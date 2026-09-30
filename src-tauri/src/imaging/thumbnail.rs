use std::{
    collections::HashMap,
    fs,
    io::{self, BufReader},
    path::{Path, PathBuf},
    sync::{atomic::AtomicU64, atomic::AtomicUsize, atomic::Ordering, Arc, Mutex},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use serde::Serialize;
use tauri::{AppHandle, Manager};

const THUMBNAIL_BOUND: u32 = 128;
const STALE_SESSION_AGE_NANOS: u128 = 24 * 60 * 60 * 1_000_000_000;

#[derive(Debug, Clone)]
struct CachedThumbnail {
    path: PathBuf,
    width: u32,
    height: u32,
}

#[derive(Debug)]
struct ThumbnailCacheInner {
    directory: PathBuf,
    entries: Mutex<HashMap<PathBuf, CachedThumbnail>>,
    next_filename: AtomicU64,
}

impl Drop for ThumbnailCacheInner {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[derive(Clone, Debug)]
pub struct ThumbnailCache(Arc<ThumbnailCacheInner>);

impl ThumbnailCache {
    pub fn new(app: &AppHandle) -> io::Result<Self> {
        let cache_directory = app.path().app_cache_dir().map_err(io::Error::other)?;
        let thumbnail_root = cache_directory.join("thumbnails");
        let _ = prune_stale_thumbnail_sessions(&thumbnail_root, SystemTime::now());
        Self::new_in(thumbnail_root)
    }

    fn new_in(root: PathBuf) -> io::Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let directory = root.join(format!("{}-{timestamp}", std::process::id()));
        fs::create_dir_all(&directory)?;

        Ok(Self(Arc::new(ThumbnailCacheInner {
            directory,
            entries: Mutex::new(HashMap::new()),
            next_filename: AtomicU64::new(0),
        })))
    }

    fn cached(&self, source: &Path) -> Option<CachedThumbnail> {
        self.0
            .entries
            .lock()
            .expect("thumbnail cache lock")
            .get(source)
            .filter(|thumbnail| thumbnail.path.is_file())
            .cloned()
    }

    fn destination(&self) -> PathBuf {
        let number = self.0.next_filename.fetch_add(1, Ordering::Relaxed);
        self.0.directory.join(format!("thumbnail-{number}.png"))
    }

    fn insert(&self, source: PathBuf, thumbnail: CachedThumbnail) {
        self.0
            .entries
            .lock()
            .expect("thumbnail cache lock")
            .insert(source, thumbnail);
    }

    pub fn release(&self, sources: &[PathBuf]) -> Result<(), ThumbnailError> {
        let thumbnails = {
            let mut entries = self.0.entries.lock().expect("thumbnail cache lock");
            sources
                .iter()
                .filter_map(|source| entries.remove(source))
                .collect::<Vec<_>>()
        };

        for thumbnail in thumbnails {
            remove_cache_file(&thumbnail.path)?;
        }

        Ok(())
    }

    pub fn clear(&self) -> Result<(), ThumbnailError> {
        let thumbnails = self
            .0
            .entries
            .lock()
            .expect("thumbnail cache lock")
            .drain()
            .map(|(_, thumbnail)| thumbnail)
            .collect::<Vec<_>>();

        for thumbnail in thumbnails {
            remove_cache_file(&thumbnail.path)?;
        }

        Ok(())
    }
}

fn prune_stale_thumbnail_sessions(root: &Path, now: SystemTime) -> io::Result<()> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let now_nanos = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let Some(created_nanos) = session_timestamp(&entry.file_name()) else {
            continue;
        };
        if now_nanos.saturating_sub(created_nanos) < STALE_SESSION_AGE_NANOS {
            continue;
        }
        let _ = fs::remove_dir_all(entry.path());
    }

    Ok(())
}

fn session_timestamp(name: &std::ffi::OsStr) -> Option<u128> {
    let name = name.to_str()?;
    let (process_id, timestamp) = name.split_once('-')?;
    process_id.parse::<u32>().ok()?;
    timestamp.parse::<u128>().ok()
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThumbnailErrorCode {
    FileNotFound,
    PermissionDenied,
    DecodeFailed,
    CacheUnavailable,
    WriteFailed,
    InvalidPath,
    Internal,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailError {
    pub code: ThumbnailErrorCode,
    pub message: String,
}

impl ThumbnailError {
    fn new(code: ThumbnailErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn internal() -> Self {
        Self::new(
            ThumbnailErrorCode::Internal,
            "The thumbnail task could not be completed.",
        )
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ThumbnailResult {
    Ready {
        path: String,
        #[serde(rename = "thumbnailPath")]
        thumbnail_path: String,
        width: u32,
        height: u32,
    },
    Error {
        path: String,
        error: ThumbnailError,
    },
}

pub fn generate_thumbnail_files(
    paths: Vec<PathBuf>,
    cache: ThumbnailCache,
) -> Vec<ThumbnailResult> {
    if paths.is_empty() {
        return Vec::new();
    }

    let worker_count = thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1)
        .min(4)
        .min(paths.len());
    let next_index = AtomicUsize::new(0);
    let results = Mutex::new(
        std::iter::repeat_with(|| None)
            .take(paths.len())
            .collect::<Vec<Option<ThumbnailResult>>>(),
    );

    thread::scope(|scope| {
        for _ in 0..worker_count {
            scope.spawn(|| loop {
                let index = next_index.fetch_add(1, Ordering::Relaxed);
                let Some(path) = paths.get(index) else {
                    break;
                };
                let result = thumbnail_result(path, &cache);
                results.lock().expect("thumbnail results lock")[index] = Some(result);
            });
        }
    });

    results
        .into_inner()
        .expect("thumbnail results lock")
        .into_iter()
        .map(|result| result.expect("every thumbnail produces a result"))
        .collect()
}

fn thumbnail_result(source: &Path, cache: &ThumbnailCache) -> ThumbnailResult {
    let path = source.to_string_lossy().into_owned();
    match generate_thumbnail(source, cache) {
        Ok(thumbnail) => match thumbnail.path.to_str() {
            Some(thumbnail_path) => ThumbnailResult::Ready {
                path,
                thumbnail_path: thumbnail_path.to_owned(),
                width: thumbnail.width,
                height: thumbnail.height,
            },
            None => ThumbnailResult::Error {
                path,
                error: ThumbnailError::new(
                    ThumbnailErrorCode::InvalidPath,
                    "The thumbnail path cannot be represented safely.",
                ),
            },
        },
        Err(error) => ThumbnailResult::Error { path, error },
    }
}

fn generate_thumbnail(
    source: &Path,
    cache: &ThumbnailCache,
) -> Result<CachedThumbnail, ThumbnailError> {
    if let Some(cached) = cache.cached(source) {
        return Ok(cached);
    }

    let file = fs::File::open(source).map_err(map_read_error)?;
    let reader = ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .map_err(map_read_error)?;
    match reader.format() {
        Some(ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP) => {}
        _ => return Err(decode_error()),
    }

    let mut decoder = reader.into_decoder().map_err(|_| decode_error())?;
    let orientation = decoder.orientation().map_err(|_| decode_error())?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|_| decode_error())?;
    image.apply_orientation(orientation);

    let thumbnail = make_thumbnail(&image);
    let destination = cache.destination();
    thumbnail
        .save_with_format(&destination, ImageFormat::Png)
        .map_err(|_| {
            ThumbnailError::new(
                ThumbnailErrorCode::WriteFailed,
                "The thumbnail could not be saved.",
            )
        })?;

    let cached = CachedThumbnail {
        path: destination,
        width: thumbnail.width(),
        height: thumbnail.height(),
    };
    cache.insert(source.to_path_buf(), cached.clone());
    Ok(cached)
}

fn make_thumbnail(image: &DynamicImage) -> DynamicImage {
    if image.width() <= THUMBNAIL_BOUND && image.height() <= THUMBNAIL_BOUND {
        image.clone()
    } else {
        image.thumbnail(THUMBNAIL_BOUND, THUMBNAIL_BOUND)
    }
}

fn decode_error() -> ThumbnailError {
    ThumbnailError::new(
        ThumbnailErrorCode::DecodeFailed,
        "A preview could not be generated.",
    )
}

fn map_read_error(error: io::Error) -> ThumbnailError {
    match error.kind() {
        io::ErrorKind::NotFound => ThumbnailError::new(
            ThumbnailErrorCode::FileNotFound,
            "The image is no longer available.",
        ),
        io::ErrorKind::PermissionDenied => ThumbnailError::new(
            ThumbnailErrorCode::PermissionDenied,
            "PixChisel cannot read this image for preview.",
        ),
        _ => decode_error(),
    }
}

fn remove_cache_file(path: &Path) -> Result<(), ThumbnailError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(ThumbnailError::new(
            ThumbnailErrorCode::CacheUnavailable,
            "The thumbnail cache could not be cleaned.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, UNIX_EPOCH},
    };

    use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};

    use super::{
        generate_thumbnail_files, make_thumbnail, prune_stale_thumbnail_sessions, ThumbnailCache,
        ThumbnailResult,
    };

    static NEXT_TEST_CACHE: AtomicU64 = AtomicU64::new(0);

    fn test_cache() -> (PathBuf, ThumbnailCache) {
        let number = NEXT_TEST_CACHE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pixchisel-thumbnail-tests-{}-{number}",
            std::process::id()
        ));
        let cache = ThumbnailCache::new_in(root.clone()).expect("create thumbnail cache");
        (root, cache)
    }

    #[test]
    fn startup_prunes_only_stale_thumbnail_sessions() {
        let number = NEXT_TEST_CACHE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pixchisel-thumbnail-prune-tests-{}-{number}",
            std::process::id()
        ));
        let now = UNIX_EPOCH + Duration::from_secs(3 * 24 * 60 * 60);
        let stale = root.join("100-0");
        let recent_timestamp = (now - Duration::from_secs(60 * 60))
            .duration_since(UNIX_EPOCH)
            .expect("recent session timestamp")
            .as_nanos();
        let recent = root.join(format!("101-{recent_timestamp}"));
        let unrelated = root.join("user-content");
        for directory in [&stale, &recent, &unrelated] {
            fs::create_dir_all(directory).expect("create cache fixture");
            fs::write(directory.join("thumbnail.png"), b"cached").expect("write cache fixture");
        }

        prune_stale_thumbnail_sessions(&root, now).expect("prune stale thumbnail sessions");

        assert!(!stale.exists());
        assert!(recent.exists());
        assert!(unrelated.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn thumbnails_stay_inside_bounds_and_preserve_aspect_ratio() {
        let wide = DynamicImage::new_rgb8(400, 200);
        let tall = DynamicImage::new_rgb8(100, 400);

        assert_eq!(make_thumbnail(&wide).dimensions(), (128, 64));
        assert_eq!(make_thumbnail(&tall).dimensions(), (32, 128));
    }

    #[test]
    fn small_images_are_not_upscaled() {
        let image = DynamicImage::new_rgb8(40, 20);

        assert_eq!(make_thumbnail(&image).dimensions(), (40, 20));
    }

    #[test]
    fn transparent_pixels_survive_png_thumbnail_output() {
        let (root, cache) = test_cache();
        let source = root.join("transparent.png");
        fs::create_dir_all(&root).expect("create fixture directory");
        let image = ImageBuffer::from_pixel(16, 16, Rgba([20_u8, 40, 60, 0]));
        DynamicImage::ImageRgba8(image)
            .save(&source)
            .expect("write transparent fixture");

        let result = generate_thumbnail_files(vec![source], cache.clone());
        let thumbnail_path = match &result[0] {
            ThumbnailResult::Ready { thumbnail_path, .. } => thumbnail_path,
            ThumbnailResult::Error { .. } => panic!("expected ready thumbnail"),
        };
        let thumbnail = image::open(thumbnail_path).expect("read generated thumbnail");

        assert_eq!(thumbnail.get_pixel(0, 0).0[3], 0);
        drop(thumbnail);
        drop(result);
        drop(cache);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn batch_results_preserve_order_and_isolate_failures() {
        let (root, cache) = test_cache();
        fs::create_dir_all(&root).expect("create fixture directory");
        let first = root.join("first.png");
        let broken = root.join("broken.png");
        let third = root.join("third.webp");
        DynamicImage::new_rgb8(20, 10)
            .save(&first)
            .expect("write first fixture");
        fs::write(&broken, b"not an image").expect("write broken fixture");
        DynamicImage::new_rgb8(10, 20)
            .save(&third)
            .expect("write third fixture");

        let results = generate_thumbnail_files(
            vec![first.clone(), broken.clone(), third.clone()],
            cache.clone(),
        );

        assert!(
            matches!(&results[0], ThumbnailResult::Ready { path, .. } if path == first.to_str().unwrap())
        );
        assert!(
            matches!(&results[1], ThumbnailResult::Error { path, .. } if path == broken.to_str().unwrap())
        );
        assert!(
            matches!(&results[2], ThumbnailResult::Ready { path, .. } if path == third.to_str().unwrap())
        );
        drop(cache);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repeated_source_reuses_the_session_thumbnail() {
        let (root, cache) = test_cache();
        fs::create_dir_all(&root).expect("create fixture directory");
        let source = root.join("source.png");
        DynamicImage::new_rgb8(24, 12)
            .save(&source)
            .expect("write source fixture");

        let first = generate_thumbnail_files(vec![source.clone()], cache.clone());
        let second = generate_thumbnail_files(vec![source], cache.clone());
        let first_path = match &first[0] {
            ThumbnailResult::Ready { thumbnail_path, .. } => thumbnail_path,
            ThumbnailResult::Error { .. } => panic!("expected ready thumbnail"),
        };
        let second_path = match &second[0] {
            ThumbnailResult::Ready { thumbnail_path, .. } => thumbnail_path,
            ThumbnailResult::Error { .. } => panic!("expected ready thumbnail"),
        };

        assert_eq!(first_path, second_path);
        drop(cache);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ready_result_serializes_thumbnail_path_for_typescript() {
        let result = ThumbnailResult::Ready {
            path: "/images/source.png".to_owned(),
            thumbnail_path: "/cache/thumbnail.png".to_owned(),
            width: 128,
            height: 64,
        };

        let value = serde_json::to_value(result).expect("serialize thumbnail result");

        assert_eq!(value["thumbnailPath"], "/cache/thumbnail.png");
        assert!(value.get("thumbnail_path").is_none());
    }
}

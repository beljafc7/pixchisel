import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

export function useNativeFileDrop(onDrop: (paths: string[]) => void) {
  const [isDragActive, setIsDragActive] = useState(false);
  const onDropRef = useRef(onDrop);
  onDropRef.current = onDrop;

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "drop") {
          setIsDragActive(false);
          onDropRef.current(event.payload.paths);
        } else if (event.payload.type === "leave") {
          setIsDragActive(false);
        } else {
          setIsDragActive(true);
        }
      })
      .then((removeListener) => {
        if (disposed) {
          removeListener();
        } else {
          unlisten = removeListener;
        }
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return isDragActive;
}

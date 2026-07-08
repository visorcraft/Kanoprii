import { useEffect } from 'react';

type UsePageZoomInputSyncOptions = {
  currentPage: number;
  setPageInput: (value: string) => void;
  zoom: number;
  setZoomInput: (value: string) => void;
};

/** Keep editable page/zoom fields in sync when values change via buttons, wheel, etc. */
export function usePageZoomInputSync({
  currentPage,
  setPageInput,
  zoom,
  setZoomInput,
}: UsePageZoomInputSyncOptions) {
  useEffect(() => setPageInput(String(currentPage + 1)), [currentPage, setPageInput]);
  useEffect(() => setZoomInput(String(Math.round(zoom * 100))), [zoom, setZoomInput]);
}

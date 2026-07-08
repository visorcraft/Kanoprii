import { ZOOM_STEP } from '../app/constants';
import { clampZoom } from '../app/utils';
import { useAnnouncer } from '../ui/useAnnouncer';

type UsePageZoomOptions = {
  zoom: number;
  setZoom: (value: number | ((prev: number) => number)) => void;
  zoomInput: string;
  setZoomInput: (value: string) => void;
  pageInput: string;
  setPageInput: (value: string) => void;
  pageCount: number | null;
  currentPage: number;
  goToPage: (index: number) => void;
};

export function usePageZoom({
  zoom,
  setZoom,
  zoomInput,
  setZoomInput,
  pageInput,
  setPageInput,
  pageCount,
  currentPage,
  goToPage,
}: UsePageZoomOptions) {
  const { announce } = useAnnouncer();

  const zoomIn = () => {
    setZoom((z) => {
      const next = clampZoom(+(z + ZOOM_STEP).toFixed(2));
      announce(`Zoom ${Math.round(next * 100)}%`);
      return next;
    });
  };
  const zoomOut = () => {
    setZoom((z) => {
      const next = clampZoom(+(z - ZOOM_STEP).toFixed(2));
      announce(`Zoom ${Math.round(next * 100)}%`);
      return next;
    });
  };
  const resetZoom = () => {
    setZoom(1);
    announce('Zoom 100%');
  };

  const commitZoom = () => {
    const n = parseInt(zoomInput, 10);
    if (Number.isNaN(n)) {
      setZoomInput(String(Math.round(zoom * 100)));
      return;
    }
    const next = clampZoom(n / 100);
    setZoom(next);
    setZoomInput(String(Math.round(next * 100)));
    announce(`Zoom ${Math.round(next * 100)}%`);
  };

  const commitPage = () => {
    const n = parseInt(pageInput, 10);
    if (Number.isNaN(n) || pageCount === null) {
      setPageInput(String(currentPage + 1));
      return;
    }
    goToPage(n - 1);
  };

  return { zoomIn, zoomOut, resetZoom, commitZoom, commitPage };
}

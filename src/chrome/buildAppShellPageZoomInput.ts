import type { DocumentState } from '../app/useAppDocumentState';
import type { PdfPageSize } from '../app/types';

export type BuildAppShellPageZoomInputArgs = {
  doc: Pick<
    DocumentState,
    'pageCount' | 'viewMode' | 'currentPage' | 'pageInput' | 'zoom' | 'zoomInput' | 'setPageInput' | 'setZoomInput'
  >;
  modal: { pageSizes: PdfPageSize[] };
  viewer: {
    commitPage: () => void;
    goToPage: (page: number) => void;
    commitZoom: () => void;
    zoomIn: () => void;
    zoomOut: () => void;
    resetZoom: () => void;
  };
};

export function buildAppShellPageZoomInput(args: BuildAppShellPageZoomInputArgs) {
  return {
    pageInput: args.doc.pageInput,
    setPageInput: args.doc.setPageInput,
    commitPage: args.viewer.commitPage,
    goToPage: args.viewer.goToPage,
    zoomInput: args.doc.zoomInput,
    setZoomInput: args.doc.setZoomInput,
    commitZoom: args.viewer.commitZoom,
    zoomIn: args.viewer.zoomIn,
    zoomOut: args.viewer.zoomOut,
    resetZoom: args.viewer.resetZoom,
  };
}

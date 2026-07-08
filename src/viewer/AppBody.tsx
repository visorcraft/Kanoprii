import type React from 'react';
import { E2EThrowTrigger } from '../ui/E2EThrowTrigger';
import type { BirdsEyeWorkspace } from '../app/useBirdsEyeWorkspace';
import type { WorkspaceViewMode } from '../app/types';
import { BirdsEyeView } from './BirdsEyeView';
import { PdfSidebar } from './PdfSidebar';
import { ViewerMain } from './ViewerMain';

type AppBodyProps = {
  filePath: string;
  sidebar: React.ComponentProps<typeof PdfSidebar>;
  viewer: Omit<React.ComponentProps<typeof ViewerMain>, 'filePath'>;
  workspaceView: WorkspaceViewMode;
  birdsEye: BirdsEyeWorkspace;
};

export function AppBody({ filePath, sidebar, viewer, workspaceView, birdsEye }: AppBodyProps) {
  if (workspaceView === 'birdseye') {
    return (
      <div className="app-body app-body-birdseye">
        <BirdsEyeView {...birdsEye} />
        {import.meta.env.VITE_WDIO === '1' && <E2EThrowTrigger />}
      </div>
    );
  }

  return (
    <div className="app-body">
      {filePath && <PdfSidebar {...sidebar} />}
      <div className="viewer-main" tabIndex={-1} aria-label="Document viewer">
        <ViewerMain {...viewer} filePath={filePath} />
      </div>
      {import.meta.env.VITE_WDIO === '1' && <E2EThrowTrigger />}
    </div>
  );
}

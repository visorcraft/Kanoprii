import { useMemo } from 'react';
import { AppShell } from './chrome/AppShell';
import { useAppStateBootstrap } from './app/useAppStateBootstrap';
import { useAppRuntimeWiring } from './app/useAppRuntimeWiring';

function App() {
  const bootstrap = useAppStateBootstrap();
  const shell = useAppRuntimeWiring(bootstrap);
  const shellProps = useMemo(() => shell, [shell]);
  return <AppShell {...shellProps} />;
}

export default App;

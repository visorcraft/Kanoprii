import { Component, type ErrorInfo, type ReactNode } from 'react';

type ErrorBoundaryProps = {
  children: ReactNode;
  /** Rendered when an error is caught. Receives the error object and a reset callback. */
  fallback?: (error: Error | null, onReset: () => void) => ReactNode;
  /** Called when an error is caught. Use for logging/telemetry/toasts. */
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
  /** Called when the user clicks the reset action. */
  onReset?: () => void;
  /**
   * Values that, when changed, tell the boundary the underlying problem may be
   * gone and it should reset automatically (e.g. the active document path).
   */
  resetKeys?: unknown[];
};

type ErrorBoundaryState = {
  hasError: boolean;
  error: Error | null;
};

function arraysDiffer(a: unknown[] | undefined, b: unknown[] | undefined) {
  if (a === b) return false;
  if (!a || !b) return true;
  if (a.length !== b.length) return true;
  return a.some((value, index) => value !== b[index]);
}

export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    this.props.onError?.(error, errorInfo);
  }

  componentDidUpdate(prevProps: ErrorBoundaryProps) {
    if (
      this.state.hasError &&
      arraysDiffer(prevProps.resetKeys, this.props.resetKeys)
    ) {
      this.props.onReset?.();
      this.setState({ hasError: false, error: null });
    }
  }

  private handleReset = () => {
    this.props.onReset?.();
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (!this.state.hasError) {
      return this.props.children;
    }

    if (this.props.fallback) {
      return this.props.fallback(this.state.error, this.handleReset);
    }

    return (
      <div className="error-boundary" role="alert">
        <h2>Something went wrong</h2>
        {this.state.error && (
          <p className="error-boundary-message">{this.state.error.message}</p>
        )}
        <div className="error-boundary-actions">
          <button type="button" onClick={this.handleReset}>
            Try again
          </button>
          <button type="button" onClick={() => window.location.reload()}>
            Reload app
          </button>
        </div>
      </div>
    );
  }
}

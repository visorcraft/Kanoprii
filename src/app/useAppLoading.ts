import { useCallback, useEffect, useRef } from 'react';

type ToastState = { message: string; type: 'success' | 'error' } | null;

type UseAppLoadingOptions = {
  setToast: (toast: ToastState) => void;
  setLoading: (loading: boolean) => void;
};

export function useAppLoading({ setToast, setLoading }: UseAppLoadingOptions) {
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const dismissToast = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setToast(null);
  }, [setToast]);

  const showToast = useCallback((message: string, type: 'success' | 'error' = 'success') => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    setToast({ message, type });
    timeoutRef.current = setTimeout(() => setToast(null), 3000);
  }, [setToast]);

  useEffect(() => () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  const withLoading = async <T,>(fn: () => Promise<T>): Promise<T | undefined> => {
    setLoading(true);
    try {
      return await fn();
    } catch (err) {
      showToast(String(err), 'error');
      return undefined;
    } finally {
      setLoading(false);
    }
  };

  return { showToast, dismissToast, withLoading };
}

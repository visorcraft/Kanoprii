type ToastProps = {
  notification: { message: string; type: 'success' | 'error' } | null;
  onClose?: () => void;
};

export function Toast({ notification, onClose }: ToastProps) {
  if (!notification) return null;
  return (
    <div
      className={`toast toast-${notification.type}`}
      role={notification.type === 'error' ? 'alert' : 'status'}
      data-testid="toast"
    >
      <span className="toast-message">{notification.message}</span>
      {onClose && (
        <button
          type="button"
          className="toast-close"
          onClick={onClose}
          aria-label="Dismiss notification"
          data-testid="toast-close"
        >
          ×
        </button>
      )}
    </div>
  );
}

import { Component, StrictMode, type ErrorInfo, type ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './styles.css';
import { bridge, type QualificationContext } from './bridge';

const DEFAULT_QUALIFICATION: QualificationContext = { enabled: false, phase: 'visual' };

function OpenDeckBootShell({ message = 'Starting OpenDeck+…' }: { message?: string }) {
  return <div className="bootstrap-shell" data-bootstrap-state="starting">
    <div className="bootstrap-card">
      <span className="brand-mark bootstrap-brand"><span /></span>
      <div>
        <strong>OpenDeck+ 2.0.37</strong>
        <p>{message}</p>
      </div>
    </div>
  </div>;
}

interface BoundaryProps { qualification: QualificationContext; children: ReactNode }
interface BoundaryState { message: string | null }

class OpenDeckErrorBoundary extends Component<BoundaryProps, BoundaryState> {
  state: BoundaryState = { message: null };

  static getDerivedStateFromError(error: unknown): BoundaryState {
    return { message: String(error) };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    const message = `${error.message}\n${info.componentStack ?? ''}`;
    console.error('OpenDeck+ bootstrap/render failure', error, info);
    if (this.props.qualification.enabled) {
      void bridge.qualificationRecordUiMetrics({ kind: 'error', payload: { stage: 'react-render', message } }).catch(() => undefined);
    }
  }

  render() {
    if (this.state.message) {
      return <div className="bootstrap-shell bootstrap-error" data-bootstrap-state="error">
        <div className="bootstrap-card">
          <span className="brand-mark bootstrap-brand"><span /></span>
          <div><strong>OpenDeck+ could not start</strong><p>{this.state.message}</p></div>
        </div>
      </div>;
    }
    return this.props.children;
  }
}

const rootElement = document.getElementById('root');
if (!rootElement) throw new Error('OpenDeck+ root element is missing');
const root = createRoot(rootElement);
root.render(<OpenDeckBootShell />);

async function bootstrap() {
  let qualification = DEFAULT_QUALIFICATION;
  try {
    qualification = await bridge.qualificationContext();
  } catch (error) {
    console.warn('OpenDeck+ qualification context unavailable; continuing in normal mode.', error);
  }

  if (!qualification.enabled) {
    try {
      await bridge.appShow();
      await bridge.appMaximize();
      await bridge.startupVisibleAck();
    } catch (error) {
      const message = `OpenDeck+ startup window initialization failed: ${String(error)}`;
      console.error(message, error);
      root.render(<OpenDeckBootShell message={message} />);
      return;
    }
  }

  root.render(
    <StrictMode>
      <OpenDeckErrorBoundary qualification={qualification}>
        <App qualification={qualification} />
      </OpenDeckErrorBoundary>
    </StrictMode>,
  );

  if (!qualification.enabled) {
    await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
  }
}

void bootstrap();

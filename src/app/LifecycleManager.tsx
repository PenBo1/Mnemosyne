import { useLifecycle } from "./lifecycle";

export interface LifecycleManagerProps {
  onWindowClose?: () => void;
  onConfigChanged?: (config: unknown) => void;
  onError?: (error: Error) => void;
}

export function LifecycleManager({ 
  onWindowClose, 
  onConfigChanged, 
  onError 
}: LifecycleManagerProps): null {
  useLifecycle({
    onWindowClose,
    onConfigChanged,
    onError,
  });
  
  return null;
}
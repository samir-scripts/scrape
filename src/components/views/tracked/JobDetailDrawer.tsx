import { JobInspectorPanel, type JobInspectorProps } from '../../jobs/JobInspector';
export function JobDetailDrawer({isOpen, ...props}: JobInspectorProps & {isOpen: boolean}) { return isOpen ? <JobInspectorPanel {...props} /> : null; }

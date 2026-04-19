import { CheckCircle2, Loader2, XCircle } from "lucide-react";
import type { Status } from "./addAccountWizardTypes";

type AddAccountStatusAlertProps = {
  status: Status;
  message: string;
};

export function AddAccountStatusAlert({
  status,
  message,
}: AddAccountStatusAlertProps) {
  if (status === "idle" || !message) {
    return null;
  }

  const map = {
    loading: { cls: "status-info", icon: <Loader2 size={16} className="spin" /> },
    success: { cls: "status-success", icon: <CheckCircle2 size={16} /> },
    error: { cls: "status-error", icon: <XCircle size={16} /> },
  } as const;
  const { cls, icon } = map[status];

  return (
    <div className={`wiz-status ${cls}`}>
      {icon}
      <span>{message}</span>
    </div>
  );
}

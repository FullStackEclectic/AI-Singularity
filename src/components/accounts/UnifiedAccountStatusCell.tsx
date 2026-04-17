import { STATUS_LABELS } from "../../types";
import type { ApiKey, IdeAccount } from "../../types";

type UnifiedAccountItem =
  | { type: "api"; data: ApiKey }
  | { type: "ide"; data: IdeAccount };

export default function UnifiedAccountStatusCell({ item }: { item: UnifiedAccountItem }) {
  if (item.type === "api") {
    const st = item.data.status;
    let cls = "unknown";
    let text = STATUS_LABELS[st] || st;
    if (st === "valid") cls = "valid";
    else if (st === "banned" || st === "invalid" || st === "expired") cls = "invalid";
    return <span className={`status-badge ${cls}`}>{text}</span>;
  }

  const st = item.data.status;
  let cls = "unknown";
  let text = st.toUpperCase();
  if (st === "active") cls = "valid";
  else if (st === "forbidden") cls = "invalid";
  else if (st === "rate_limited" || (st as any) === "rate_limit") cls = "warning";
  return <span className={`status-badge ${cls}`}>{text}</span>;
}

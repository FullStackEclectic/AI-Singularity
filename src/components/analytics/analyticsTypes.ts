export interface HistoryPoint {
  provider_id: string;
  provider_name: string;
  balance_usd?: number;
  balance_cny?: number;
  quota_remaining?: number;
  quota_unit?: string;
  snapped_at: string;
}

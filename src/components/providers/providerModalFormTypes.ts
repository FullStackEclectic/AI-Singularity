import type { Platform, ToolTarget } from "../../types";

export interface ProviderFormState {
  name: string;
  platform: Platform;
  base_url: string;
  model_name: string;
  api_key_value: string;
  tool_targets: ToolTarget[];
  website_url: string;
  api_key_url: string;
  notes: string;
  extra_config: string;
}

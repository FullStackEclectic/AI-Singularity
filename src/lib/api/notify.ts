import { invoke } from "@tauri-apps/api/core";

export interface NotifyConfig {
  feishuEnabled: boolean;
  feishuWebhook: string;
  dingtalkEnabled: boolean;
  dingtalkWebhook: string;
  dingtalkSecret: string;
  wecomEnabled: boolean;
  wecomWebhook: string;
  emailEnabled: boolean;
  emailSmtpHost: string;
  emailSmtpPort: number;
  emailUsername: string;
  emailPassword: string;
  emailTo: string;
}

export const notify = {
  getConfig: (): Promise<NotifyConfig> => invoke("get_notify_config"),
  saveConfig: (config: NotifyConfig): Promise<void> => invoke("save_notify_config", { config }),
  testChannel: (channel: string, title: string, content: string): Promise<void> =>
    invoke("test_notify_channel", { channel, title, content }),
};

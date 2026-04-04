import { useState } from "react";
import KeysPage from "../keys/KeysPage";
import IdeAccountsPage from "../ideAccounts/IdeAccountsPage";
import UserTokenPage from "../userTokens/UserTokenPage";
import "./AccountsContainerPage.css";

type AccountTab = "keys" | "ide" | "saas";

export default function AccountsContainerPage() {
  const [activeTab, setActiveTab] = useState<AccountTab>("keys");

  return (
    <div className="accounts-container animate-fade-in">
      <div className="accounts-tabs">
        <button 
          className={activeTab === "keys" ? "active" : ""} 
          onClick={() => setActiveTab("keys")}
        >
          标准账号 (API Keys)
        </button>
        <button 
          className={activeTab === "ide" ? "active" : ""} 
          onClick={() => setActiveTab("ide")}
        >
          白嫖账号 (IDE 指纹池)
        </button>
        <button 
          className={activeTab === "saas" ? "active" : ""} 
          onClick={() => setActiveTab("saas")}
        >
          共享子号 (SaaS 下发)
        </button>
      </div>

      <div className="accounts-tab-content">
        {activeTab === "keys" && <KeysPage />}
        {activeTab === "ide" && <IdeAccountsPage />}
        {activeTab === "saas" && <UserTokenPage />}
      </div>
    </div>
  );
}

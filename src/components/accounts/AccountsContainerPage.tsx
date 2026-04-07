import { useState } from "react";
import UnifiedAccountsList from "./UnifiedAccountsList";
import SharingPage from "./SharingPage";
import "./AccountsContainerPage.css";

type AccountTab = "managed" | "shared";

export default function AccountsContainerPage() {
  const [activeTab, setActiveTab] = useState<AccountTab>("managed");

  return (
    <div className="accounts-container animate-fade-in" style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div className="accounts-tabs" style={{ padding: '0 1.5rem', paddingTop: '1rem' }}>
        <button 
          className={activeTab === "managed" ? "active" : ""} 
          onClick={() => setActiveTab("managed")}
        >
           渠道资产 (Channels)
        </button>
        <button 
          className={activeTab === "shared" ? "active" : ""} 
          onClick={() => setActiveTab("shared")}
        >
           分享额度 (Tokens)
        </button>
      </div>

      <div className="accounts-tab-content" style={{ flex: 1, overflow: 'hidden' }}>
        {activeTab === "managed" && <UnifiedAccountsList />}
        {activeTab === "shared" && <SharingPage />}
      </div>
    </div>
  );
}

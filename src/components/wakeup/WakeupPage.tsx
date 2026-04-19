import { WakeupHistoryPanel } from "./WakeupHistoryPanel";
import { WakeupOverview } from "./WakeupOverview";
import { WakeupTaskCard } from "./WakeupTaskCard";
import { WakeupVerificationPanel } from "./WakeupVerificationPanel";
import { getWakeupCategoryLabel } from "./wakeupUtils";
import { useWakeupPageState } from "./useWakeupPageState";
import "./WakeupPage.css";

export default function WakeupPage() {
  const {
    state,
    setState,
    history,
    accounts,
    taskGroupFilters,
    message,
    saving,
    loading,
    verificationSelection,
    setVerificationSelection,
    verificationModel,
    setVerificationModel,
    verificationPrompt,
    setVerificationPrompt,
    verificationCommandTemplate,
    setVerificationCommandTemplate,
    verificationTimeout,
    setVerificationTimeout,
    verificationRetryFailedTimes,
    setVerificationRetryFailedTimes,
    verificationRunning,
    verificationShowFailedOnly,
    setVerificationShowFailedOnly,
    verificationGroupFilter,
    setVerificationGroupFilter,
    verificationResultGroupFilter,
    setVerificationResultGroupFilter,
    verificationResult,
    activeVerificationRunId,
    runningTaskId,
    historyGroupFilter,
    setHistoryGroupFilter,
    activeTasks,
    failingTasks,
    accountGroupByAccountId,
    verificationGroupOptions,
    visibleVerificationAccounts,
    verificationResultGroupOptions,
    hasUngroupedVerificationResult,
    verificationVisibleItems,
    verificationFailedAccountIds,
    historyGroupOptions,
    hasUngroupedHistory,
    groupedHistory,
    getAccountGroupLabel,
    saveState,
    handleTaskChange,
    handleTaskGroupFilterChange,
    handleAddTask,
    handleDeleteTask,
    handleRunTaskNow,
    handleCopyTaskCommand,
    clearHistory,
    toggleVerificationAccount,
    runVerification,
    rerunFailedVerification,
    cancelVerification,
  } = useWakeupPageState();

  return (
    <div className="wakeup-page">
      <WakeupOverview
        state={state}
        message={message}
        saving={saving}
        activeTasksCount={activeTasks.length}
        historyCount={history.length}
        failingTasksCount={failingTasks.length}
        onAddTask={handleAddTask}
        onSave={() => saveState(state)}
        onStateChange={setState}
      />

      <div className="wakeup-layout">
        <section className="card wakeup-task-panel">
          <div className="wakeup-section-title">任务列表</div>
          {loading ? (
            <div className="wakeup-empty">加载中...</div>
          ) : state.tasks.length === 0 ? (
            <div className="wakeup-empty">当前还没有 Wakeup 任务</div>
          ) : (
            <div className="wakeup-task-list">
              {state.tasks.map((task) => (
                <WakeupTaskCard
                  key={task.id}
                  task={task}
                  accounts={accounts}
                  accountGroupByAccountId={accountGroupByAccountId}
                  verificationGroupOptions={verificationGroupOptions}
                  taskGroupFilter={taskGroupFilters[task.id] || "all"}
                  runningTaskId={runningTaskId}
                  getAccountGroupLabel={getAccountGroupLabel}
                  getWakeupCategoryLabel={getWakeupCategoryLabel}
                  onTaskGroupFilterChange={handleTaskGroupFilterChange}
                  onTaskChange={handleTaskChange}
                  onRunTaskNow={handleRunTaskNow}
                  onCopyTaskCommand={handleCopyTaskCommand}
                  onDeleteTask={handleDeleteTask}
                />
              ))}
            </div>
          )}
        </section>

        <div className="wakeup-side-stack">
          <WakeupVerificationPanel
            accounts={accounts}
            visibleAccounts={visibleVerificationAccounts}
            verificationGroupOptions={verificationGroupOptions}
            verificationSelection={verificationSelection}
            verificationModel={verificationModel}
            verificationPrompt={verificationPrompt}
            verificationCommandTemplate={verificationCommandTemplate}
            verificationTimeout={verificationTimeout}
            verificationRetryFailedTimes={verificationRetryFailedTimes}
            verificationRunning={verificationRunning}
            verificationShowFailedOnly={verificationShowFailedOnly}
            verificationGroupFilter={verificationGroupFilter}
            verificationResultGroupFilter={verificationResultGroupFilter}
            verificationResult={verificationResult}
            verificationResultGroupOptions={verificationResultGroupOptions}
            verificationVisibleItems={verificationVisibleItems}
            verificationFailedAccountIds={verificationFailedAccountIds}
            activeVerificationRunId={activeVerificationRunId}
            hasUngroupedVerificationResult={hasUngroupedVerificationResult}
            getAccountGroupLabel={getAccountGroupLabel}
            getWakeupCategoryLabel={getWakeupCategoryLabel}
            onVerificationGroupFilterChange={setVerificationGroupFilter}
            onVerificationSelectionChange={setVerificationSelection}
            onVerificationModelChange={setVerificationModel}
            onVerificationPromptChange={setVerificationPrompt}
            onVerificationCommandTemplateChange={setVerificationCommandTemplate}
            onVerificationTimeoutChange={setVerificationTimeout}
            onVerificationRetryFailedTimesChange={setVerificationRetryFailedTimes}
            onToggleVerificationAccount={toggleVerificationAccount}
            onRunVerification={() => runVerification()}
            onCancelVerification={cancelVerification}
            onVerificationShowFailedOnlyChange={setVerificationShowFailedOnly}
            onVerificationResultGroupFilterChange={setVerificationResultGroupFilter}
            onRerunFailedVerification={rerunFailedVerification}
          />
          <WakeupHistoryPanel
            historyCount={history.length}
            historyGroupFilter={historyGroupFilter}
            historyGroupOptions={historyGroupOptions}
            hasUngroupedHistory={hasUngroupedHistory}
            groupedHistory={groupedHistory}
            getAccountGroupLabel={getAccountGroupLabel}
            getWakeupCategoryLabel={getWakeupCategoryLabel}
            onHistoryGroupFilterChange={setHistoryGroupFilter}
            onClearHistory={clearHistory}
          />
        </div>
      </div>
    </div>
  );
}

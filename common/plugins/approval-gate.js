import { readFileSync } from "node:fs"
import { join } from "node:path"

// MVP gate: deny writes AND shell while no human-approved task exists.
// Intentional strictness: planning is conversational; no filesystem writes.
export const ApprovalGate = async ({ directory }) => {
  const approvalPath = join(directory, ".opencode", "workflow-approval.json")
  const guardedTools = new Set(["write", "edit", "apply_patch", "multiedit", "bash"])

  return {
    "tool.execute.before": async (input) => {
      if (!guardedTools.has(input.tool)) return
      let approved = false
      try {
        const data = JSON.parse(readFileSync(approvalPath, "utf8"))
        approved = data?.approved === true &&
          typeof data?.task === "string" &&
          /^TASK-[A-Za-z0-9_-]+$/.test(data.task)
      } catch (_) {
        // No approval file / invalid JSON -> FAIL CLOSED.
      }
      if (!approved) {
        throw new Error(
          `PLAN_GATE: ${input.tool} blocked. First run implementation-breakdown and refine-plan; ` +
          `then ask the human to approve a specific TASK in a separate terminal.`
        )
      }
    },
  }
}

from core.profile_management.implementations.python.runtime import mcp_runtime

PROFILES = {"backend-go": "go-backend", "frontend-react": "react-frontend"}
RULE = ".opencode/workflow-rules.md"
GATE = ".opencode/workflow-approval.json"
STATE = mcp_runtime.STATE
IGNORE_LINE = GATE
SCHEMA = "https://opencode.ai/config.json"

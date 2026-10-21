import { Plugin } from "@opencode/plugin"
import { basename } from "node:path"

// Add file names here. Each name is protected in every repository directory.
const protectedFiles = new Set(["README.md", "AGENTS.md"])

export default Plugin.define({
  id: "ai-profiles.documentation-guard",
  async setup(ctx) {
    await ctx.permission.hook("evaluate", (event) => {
      if (event.action !== "edit" || event.effect === "deny") return

      const files = event.resources.filter((resource) =>
        protectedFiles.has(basename(resource.replaceAll("\\", "/"))),
      )
      if (files.length === 0) return

      event.effect = "ask"
      event.message = `Zmiana chronionych plików (${files.join(", ")}) wymaga zgody człowieka. Zatwierdź tę operację lub odrzuć z propozycją innego miejsca na opis.`
    })
  },
})

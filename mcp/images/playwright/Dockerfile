FROM node:22.16.0-bookworm-slim
ARG TARGETARCH
LABEL org.opencontainers.image.title="ai-profiles Playwright MCP" \
      org.opencontainers.image.version="0.1.0" \
      org.opencontainers.image.source="https://github.com/kpdve-ai-workflow/ai-profiles"
WORKDIR /opt/playwright
ENV PLAYWRIGHT_BROWSERS_PATH=/opt/playwright-browsers
RUN npm install --no-save --package-lock=false --omit=dev \
      @playwright/mcp@0.0.83 axe-core@4.14.0 \
 && ./node_modules/.bin/playwright install --with-deps chromium \
 && apt-get update && apt-get install -y --no-install-recommends ca-certificates util-linux \
 && rm -rf /var/lib/apt/lists/* \
 && chmod -R a+rX /opt/playwright /opt/playwright-browsers \
 && mkdir -p /opt/notices \
 && cp node_modules/@playwright/mcp/LICENSE /opt/notices/playwright-mcp-LICENSE \
 && cp node_modules/axe-core/LICENSE /opt/notices/axe-core-LICENSE
RUN apt-get update \
 && apt-get install -y --no-install-recommends libnss3-tools \
 && rm -rf /var/lib/apt/lists/*
COPY entrypoint.sh /usr/local/bin/playwright-entrypoint
RUN chmod 755 /usr/local/bin/playwright-entrypoint \
 && node -e "const p=require('./node_modules/@playwright/mcp/package.json');const a=require('./node_modules/axe-core/package.json');require('fs').writeFileSync('/opt/playwright/versions.txt', 'Playwright MCP '+p.version+'\\nPlaywright '+require('./node_modules/playwright/package.json').version+'\\naxe-core '+a.version+'\\n');"
ENTRYPOINT ["/usr/local/bin/playwright-entrypoint"]

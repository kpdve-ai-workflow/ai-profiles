FROM node:24.21.0-bookworm-slim
RUN npm install --global --omit=dev @automattic/mcp-wordpress-remote@0.4.0 \
    && npm cache clean --force
ENV OAUTH_ENABLED=false NODE_OPTIONS=--use-system-ca LOG_LEVEL=-1
USER node
WORKDIR /home/node
ENTRYPOINT ["mcp-wordpress-remote"]

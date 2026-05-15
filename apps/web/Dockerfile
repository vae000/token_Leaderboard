# ── Token Leaderboard Web ──────────────────────────────────
# 构建: docker build -f apps/web/Dockerfile .

FROM swr.cn-north-4.myhuaweicloud.com/ddn-k8s/docker.io/library/node:22-alpine AS builder
WORKDIR /app

COPY apps/web/package.json apps/web/pnpm-lock.yaml ./

RUN npm config set registry https://registry.npmmirror.com \
    && npm install -g pnpm@11.1.2 \
    && pnpm config set registry https://registry.npmmirror.com \
    && pnpm config set fetch-retries 5 \
    && pnpm config set fetch-timeout 600000 \
    && pnpm install --frozen-lockfile --ignore-scripts

COPY apps/web/ .
RUN CI=true pnpm build

FROM swr.cn-north-4.myhuaweicloud.com/ddn-k8s/docker.io/library/node:22-alpine
WORKDIR /app

COPY --from=builder /app/.next ./.next
COPY --from=builder /app/package.json ./
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/public ./public
COPY --from=builder /app/next.config.ts ./
COPY --from=builder /app/tsconfig.json ./
COPY --from=builder /app/next-env.d.ts ./

EXPOSE 3000
CMD ["npx", "next", "start"]

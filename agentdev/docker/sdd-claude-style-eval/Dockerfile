FROM rust:nightly-bookworm AS llman_builder

WORKDIR /repo
COPY . .

# Build llman CLI (release).
RUN cargo build --release


FROM node:22-bookworm AS runtime

ARG APT_MIRROR_DEBIAN="http://deb.debian.org/debian"
ARG APT_MIRROR_SECURITY="http://deb.debian.org/debian-security"
ARG NPM_REGISTRY="https://registry.npmjs.org/"
ARG PIP_INDEX_URL="https://pypi.org/simple"
ARG PIP_TRUSTED_HOST=""
ARG PROMPTFOO_VERSION="0.121.2"

ENV PIP_INDEX_URL="${PIP_INDEX_URL}"
ENV PIP_TRUSTED_HOST="${PIP_TRUSTED_HOST}"
ENV PIP_DISABLE_PIP_VERSION_CHECK="1"

# Switch apt mirrors (supports Aliyun mirrors via build args).
RUN set -eux; \
  if [ -f /etc/apt/sources.list.d/debian.sources ]; then \
    sed -i "s|http://deb.debian.org/debian|${APT_MIRROR_DEBIAN}|g" /etc/apt/sources.list.d/debian.sources; \
    sed -i "s|http://deb.debian.org/debian-security|${APT_MIRROR_SECURITY}|g" /etc/apt/sources.list.d/debian.sources; \
    sed -i "s|http://security.debian.org/debian-security|${APT_MIRROR_SECURITY}|g" /etc/apt/sources.list.d/debian.sources; \
  elif [ -f /etc/apt/sources.list ]; then \
    sed -i "s|http://deb.debian.org/debian|${APT_MIRROR_DEBIAN}|g" /etc/apt/sources.list; \
    sed -i "s|http://deb.debian.org/debian-security|${APT_MIRROR_SECURITY}|g" /etc/apt/sources.list; \
    sed -i "s|http://security.debian.org/debian-security|${APT_MIRROR_SECURITY}|g" /etc/apt/sources.list; \
  fi; \
  apt-get update; \
  apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    jq \
    python3 \
    ripgrep; \
  rm -rf /var/lib/apt/lists/*

# npm mirror (supports Aliyun npm mirror via build args).
RUN set -eux; \
  npm config set registry "${NPM_REGISTRY}"; \
  npm install -g "promptfoo@${PROMPTFOO_VERSION}"

WORKDIR /repo

COPY --from=llman_builder /repo/agentdev /repo/agentdev
COPY --from=llman_builder /repo/scripts /repo/scripts
COPY --from=llman_builder /repo/target/release/llman /usr/local/bin/llman

# Install local deps needed by promptfoo's anthropic:claude-agent-sdk provider.
RUN set -eux; \
  cd /repo/agentdev/promptfoo; \
  npm install --no-audit --no-fund

ENV PATH="/usr/local/bin:${PATH}"

# Default to showing help.
CMD ["bash", "scripts/sdd-claude-style-eval.sh", "--help"]

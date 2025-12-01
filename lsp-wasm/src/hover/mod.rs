//! Hover documentation for Runefile LSP

use crate::parser::types::*;
use wasm_bindgen::prelude::*;

/// Hover provider for Runefile
#[wasm_bindgen]
pub struct HoverProvider;

#[wasm_bindgen]
impl HoverProvider {
    /// Create a new hover provider
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Get hover information at position (works offline)
    #[wasm_bindgen(js_name = getHover)]
    pub fn get_hover(&self, content: &str, line: u32, character: u32) -> String {
        let lines: Vec<&str> = content.lines().collect();
        
        if (line as usize) >= lines.len() {
            return "null".to_string();
        }

        let current_line = lines[line as usize];
        let trimmed = current_line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return "null".to_string();
        }

        // Get the word at cursor position
        let word = self.get_word_at_position(current_line, character as usize);
        
        // Check if it's an instruction keyword
        let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
        let instruction = parts.get(0).unwrap_or(&"").to_uppercase();

        // If cursor is on the instruction keyword
        if let Some(doc) = self.get_instruction_documentation(&instruction) {
            let result = HoverResult {
                contents: doc,
                range: Some(Range {
                    start: Position { line, character: 0 },
                    end: Position { line, character: instruction.len() as u32 },
                }),
            };
            return serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string());
        }

        // Check for common patterns in arguments
        if let Some(doc) = self.get_pattern_documentation(&word) {
            let result = HoverResult {
                contents: doc,
                range: None,
            };
            return serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string());
        }

        "null".to_string()
    }

    fn get_word_at_position(&self, line: &str, position: usize) -> String {
        let chars: Vec<char> = line.chars().collect();
        if position >= chars.len() {
            return String::new();
        }

        let mut start = position;
        let mut end = position;

        // Find word boundaries
        while start > 0 && !chars[start - 1].is_whitespace() && chars[start - 1] != '=' {
            start -= 1;
        }
        while end < chars.len() && !chars[end].is_whitespace() && chars[end] != '=' {
            end += 1;
        }

        chars[start..end].iter().collect()
    }

    fn get_instruction_documentation(&self, instruction: &str) -> Option<String> {
        let doc = match instruction {
            "FROM" => r#"# FROM

Sets the base image for subsequent instructions.

**Syntax:**
```dockerfile
FROM <image>[:<tag>] [AS <name>]
```

**Examples:**
```dockerfile
FROM alpine:latest
FROM ubuntu:22.04 AS builder
FROM scratch
```

**Notes:**
- Must be the first instruction (except ARG)
- Can appear multiple times for multi-stage builds
- Use `scratch` for minimal images"#,

            "RUN" => r#"# RUN

Executes commands in a new layer on top of the current image.

**Syntax:**
```dockerfile
RUN <command>                    # Shell form
RUN ["executable", "param1"]     # Exec form
```

**Examples:**
```dockerfile
RUN apt-get update && apt-get install -y curl
RUN ["pip", "install", "flask"]
```

**Best Practices:**
- Combine related commands with `&&`
- Clean up in the same layer to reduce image size
- Use exec form for better signal handling"#,

            "COPY" => r#"# COPY

Copies files/directories from build context to the image.

**Syntax:**
```dockerfile
COPY [--chown=<user>:<group>] <src>... <dest>
COPY [--from=<name>] <src>... <dest>
```

**Examples:**
```dockerfile
COPY . /app
COPY --from=builder /app/bin /usr/local/bin
COPY --chown=node:node package*.json ./
```

**Notes:**
- Use `--from` for multi-stage builds
- Wildcards are supported
- Preserves file permissions"#,

            "ADD" => r#"# ADD

Similar to COPY but with additional features.

**Syntax:**
```dockerfile
ADD [--chown=<user>:<group>] <src>... <dest>
```

**Features:**
- Auto-extracts local tar archives
- Can fetch remote URLs
- Supports `--checksum` for verification

**Note:** Prefer COPY unless you need ADD's features"#,

            "CMD" => r#"# CMD

Provides default command for container execution.

**Syntax:**
```dockerfile
CMD ["executable","param1"]    # Exec form (preferred)
CMD command param1             # Shell form
CMD ["param1","param2"]        # Default params for ENTRYPOINT
```

**Notes:**
- Only one CMD per Dockerfile (last one wins)
- Can be overridden at runtime
- Use exec form for proper signal handling"#,

            "ENTRYPOINT" => r#"# ENTRYPOINT

Configures the container to run as an executable.

**Syntax:**
```dockerfile
ENTRYPOINT ["executable", "param1"]   # Exec form
ENTRYPOINT command param1             # Shell form
```

**Examples:**
```dockerfile
ENTRYPOINT ["python", "app.py"]
ENTRYPOINT ["/docker-entrypoint.sh"]
```

**Notes:**
- Use with CMD for default arguments
- Exec form recommended for signal handling"#,

            "ENV" => r#"# ENV

Sets environment variables.

**Syntax:**
```dockerfile
ENV <key>=<value> ...
ENV <key> <value>
```

**Examples:**
```dockerfile
ENV NODE_ENV=production
ENV PATH="/app/bin:$PATH"
ENV MY_VAR=value OTHER_VAR=other
```

**Notes:**
- Available during build and at runtime
- Can be overridden with `docker run -e`"#,

            "EXPOSE" => r#"# EXPOSE

Documents which ports the container listens on.

**Syntax:**
```dockerfile
EXPOSE <port>[/<protocol>]
```

**Examples:**
```dockerfile
EXPOSE 80
EXPOSE 443/tcp
EXPOSE 8080/udp
```

**Note:** This is documentation only; use `-p` to publish ports"#,

            "WORKDIR" => r#"# WORKDIR

Sets the working directory for subsequent instructions.

**Syntax:**
```dockerfile
WORKDIR /path/to/dir
```

**Examples:**
```dockerfile
WORKDIR /app
WORKDIR /src/myapp
```

**Notes:**
- Creates directory if it doesn't exist
- Can use environment variables
- Relative paths are relative to previous WORKDIR"#,

            "USER" => r#"# USER

Sets the user for subsequent instructions and container runtime.

**Syntax:**
```dockerfile
USER <user>[:<group>]
USER <UID>[:<GID>]
```

**Examples:**
```dockerfile
USER node
USER 1000:1000
```

**Best Practice:** Run as non-root for security"#,

            "VOLUME" => r#"# VOLUME

Creates a mount point for externally mounted volumes.

**Syntax:**
```dockerfile
VOLUME ["/data"]
VOLUME /var/log /var/db
```

**Notes:**
- Data persists beyond container lifecycle
- Cannot specify host directory in Dockerfile"#,

            "ARG" => r#"# ARG

Defines a build-time variable.

**Syntax:**
```dockerfile
ARG <name>[=<default value>]
```

**Examples:**
```dockerfile
ARG VERSION=latest
ARG BUILD_DATE
```

**Usage:**
```bash
docker build --build-arg VERSION=1.0 .
```

**Note:** ARG values don't persist in the final image"#,

            "LABEL" => r#"# LABEL

Adds metadata to an image.

**Syntax:**
```dockerfile
LABEL <key>=<value> <key>=<value> ...
```

**Examples:**
```dockerfile
LABEL version="1.0"
LABEL maintainer="team@example.com"
LABEL org.opencontainers.image.source="https://github.com/..."
```"#,

            "HEALTHCHECK" => r#"# HEALTHCHECK

Tells Docker how to test if the container is still healthy.

**Syntax:**
```dockerfile
HEALTHCHECK [OPTIONS] CMD command
HEALTHCHECK NONE
```

**Options:**
- `--interval=30s` - Time between checks
- `--timeout=30s` - Timeout for check
- `--start-period=0s` - Initial grace period
- `--retries=3` - Consecutive failures before unhealthy

**Example:**
```dockerfile
HEALTHCHECK --interval=30s CMD curl -f http://localhost/ || exit 1
```"#,

            "SHELL" => r#"# SHELL

Sets the default shell for shell-form commands.

**Syntax:**
```dockerfile
SHELL ["executable", "parameters"]
```

**Examples:**
```dockerfile
SHELL ["/bin/bash", "-c"]
SHELL ["powershell", "-Command"]
```

**Note:** Affects RUN, CMD, and ENTRYPOINT shell forms"#,

            "STOPSIGNAL" => r#"# STOPSIGNAL

Sets the signal to stop the container.

**Syntax:**
```dockerfile
STOPSIGNAL signal
```

**Examples:**
```dockerfile
STOPSIGNAL SIGTERM
STOPSIGNAL SIGKILL
STOPSIGNAL 9
```"#,

            _ => return None,
        };

        Some(doc.to_string())
    }

    fn get_pattern_documentation(&self, word: &str) -> Option<String> {
        match word {
            "--from" => Some("Copy from a previous build stage".to_string()),
            "--chown" => Some("Set file ownership (user:group)".to_string()),
            "--chmod" => Some("Set file permissions".to_string()),
            "scratch" => Some("Empty base image for minimal containers".to_string()),
            "alpine" => Some("Minimal Linux distribution (~5MB)".to_string()),
            "AS" => Some("Name this build stage for multi-stage builds".to_string()),
            _ => None,
        }
    }
}

impl Default for HoverProvider {
    fn default() -> Self {
        Self::new()
    }
}

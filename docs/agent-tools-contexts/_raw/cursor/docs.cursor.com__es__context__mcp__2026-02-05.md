---
source_url: https://docs.cursor.com/es/context/mcp
title: Cursor Docs - MCP（西语）
fetched_at: 2026-02-05T23:26:33+08:00
version_or_last_updated: unknown
---

## 配置位置（原文摘录）

> Crea `.cursor/mcp.json` en tu proyecto para herramientas específicas del proyecto.
>
> Crea `~/.cursor/mcp.json` en tu directorio personal para tener las herramientas disponibles en cualquier lugar.

## 插值字段与语法（原文摘录）

> Cursor resuelve variables en estos campos: `command`, `args`, `env`, `url` y `headers`.
> Sintaxis soportada:
>
>   * `${env:NAME}` Variable de entorno
>   * `${userHome}` Ruta a tu directorio personal
>   * `${workspaceFolder}` Raíz del proyecto (la carpeta que contiene `.cursor/mcp.json`)

## 禁用 MCP server（原文摘录）

> 1. Abre Settings (`Ctrl+Shift+J`)
> 2. Ve a Features → Model Context Protocol
> 3. Haz clic en el interruptor junto a cualquier servidor para habilitarlo/deshabilitarlo
> Los servidores deshabilitados no se cargarán ni aparecerán en el chat.

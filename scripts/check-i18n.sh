#!/bin/bash

echo "🔍 检查项目 i18n 状态..."

# 检查是否有硬编码的英文字符串
echo ""
echo "📝 检查 Rust 源码中的硬编码英文字符串..."
echo "     (输出格式: path/to/file.rs:line_number:matched_line)"

# 排除已知的常量定义、测试代码和配置
# grep -rn already provides file and line number.
HARDCODED_STRINGS=$(grep -rn '"[^"]*[a-zA-Z][^"]*"' src/ \
    --include="*.rs" \
    | grep -v "const " \
    | grep -v "#\[cfg(test)\]" \
    | grep -v "test_" \
    | grep -v 't!(' \
    | grep -v ".rs:.*:.*//" \
    | grep -v ".join(" \
    | grep -v "format!(" \
    | grep -v "path" \
    | grep -v ".mdc" \
    | grep -v ".git" \
    | grep -v "ENV_" \
    | grep -v "APP_NAME" \
    | grep -v "CURSOR" \
    | grep -v ".txt" \
    | grep -v "prompt" \
    | grep -v "zh-CN" \
    | grep -v "en" \
    | grep -v "locales" \
    | grep -v "DEBUG\|INFO\|WARN\|ERROR")

if [ -n "$HARDCODED_STRINGS" ]; then
    echo "⚠️  发现可能需要 i18n 的硬编码字符串 (文件:行号:内容):"
    echo "$HARDCODED_STRINGS"
else
    echo "✅ 没有发现明显的硬编码英文字符串。"
fi

# 检查 println! 和 eprintln! 是否都使用了 t! 宏
echo ""
echo "📤 检查输出语句是否使用了 i18n..."
echo "     (输出格式: path/to/file.rs:line_number:matched_line)"

PRINT_WITHOUT_T=$(grep -rn "println!\|eprintln!" src/ \
    --include="*.rs" \
    | grep -v 't!(' \
    | grep -v "#\[cfg(test)\]" \
    | grep -v "test_")

if [ -n "$PRINT_WITHOUT_T" ]; then
    echo "⚠️  发现未使用 t! 宏的输出语句, 请根据语义判断, 仅emoji不需要i18n (文件:行号:内容):"
    echo "$PRINT_WITHOUT_T"
else
    echo "✅ 所有输出语句似乎都使用了 i18n (t! 宏)。"
fi

# 检查错误消息
echo ""
echo "❌ 检查错误处理是否使用了本地化..."
echo "     (输出格式: path/to/file.rs:line_number:matched_line)"

ERROR_WITHOUT_LOCALIZED=$(grep -rn "eprintln!\|panic!" src/ \
    --include="*.rs" \
    | grep -v "display_localized\|t!(" \
    | grep -v "#\[cfg(test)\]" \
    | grep -v "test_")

if [ -n "$ERROR_WITHOUT_LOCALIZED" ]; then
    echo "⚠️  发现可能未本地化的错误输出 (文件:行号:内容):"
    echo "$ERROR_WITHOUT_LOCALIZED"
else
    echo "✅ 错误处理似乎都使用了本地化。"
fi

# 检查 locales 文件中的翻译完整性
echo ""
echo "🌐 检查翻译文件完整性..."

LOCALES_FILE="locales/app.yml" # 定义文件路径变量

if [ ! -f "$LOCALES_FILE" ]; then
    echo "⚠️  翻译文件 $LOCALES_FILE 未找到。"
else
    echo "ℹ️  正在分析 $LOCALES_FILE ..."

    # Function to extract keys from a language section in a YAML file.
    # Assumes keys are indented and followed by a colon.
    # Handles simple flat keys and keys with dots (e.g., parent.child).
    get_lang_keys() {
        local lang_code="$1"
        local file="$2"
        # awk script:
        # BEGIN: Initialize in_lang_section to 0 (false).
        # $0 == lang_code ":": If the current line is exactly the language code followed by a colon (e.g., "en:"),
        #   set in_lang_section to 1 (true) and skip to the next line.
        # in_lang_section && NF > 0 && !/^[[:space:]]/ && $0 !~ /^#/: If in a language section,
        #   the line is not empty, is not indented, and is not a comment,
        #   it means we've reached the end of the current language's keys (e.g., another language tag).
        #   So, set in_lang_section to 0.
        # in_lang_section && /^[[:space:]]+[^:]+:.*/: If in a language section and the line is indented
        #   and contains a key (text followed by a colon), then extract the key.
        #   - key = $0: Copy the line.
        #   - sub(/^[[:space:]]+/, "", key): Remove leading spaces.
        #   - sub(/:.*/, "", key): Remove the colon and everything after it.
        #   - if (key != "") print key: If the extracted key is not empty, print it.
        awk -v lang_code="$lang_code" '
            BEGIN { in_lang_section = 0; }
            $0 == lang_code ":" { in_lang_section = 1; next; }
            in_lang_section && NF > 0 && !/^[[:space:]]/ && $0 !~ /^#/ { in_lang_section = 0; }
            in_lang_section && /^[[:space:]]+[^:]+:.*/ {
                key = $0;
                sub(/^[[:space:]]+/, "", key);
                sub(/:.*/, "", key);
                if (key != "") print key;
            }
        ' "$file" | sort -u # Sort and unique to handle any duplicates or ordering issues
    }

    EN_KEYS=$(get_lang_keys "en" "$LOCALES_FILE")
    ZH_KEYS=$(get_lang_keys "zh-CN" "$LOCALES_FILE")

    MISSING_IN_ZH_DETAILS=""
    EN_KEY_COUNT=0

    for key in $EN_KEYS; do
        EN_KEY_COUNT=$((EN_KEY_COUNT + 1))
        if ! echo "$ZH_KEYS" | grep -Fxq "$key"; then
            # Try to find the line number of the English key definition
            EN_SECTION_START_LINE=$(grep -nm1 "^en:" "$LOCALES_FILE" | cut -d: -f1)
            KEY_LINE_IN_FILE=""
            if [ -n "$EN_SECTION_START_LINE" ]; then
                # awk to find key:
                # NR > start_line: Process lines after the 'en:' tag.
                # $0 ~ key_pattern: If line matches the key pattern (e.g., "  actual_key:").
                #   print NR; exit: Print line number and exit awk.
                # !/^[[:space:]]/ && $0 !~ /^#/: If line is not indented (and not a comment),
                #   it's likely a new section, so stop searching in this block.
                KEY_LINE_IN_FILE=$(awk -v start_line="$EN_SECTION_START_LINE" -v key_pattern="^[[:space:]]+${key}:" \
                                    'NR > start_line { if ($0 ~ key_pattern) { print NR; exit } if (!/^[[:space:]]/ && $0 !~ /^#/) { exit } }' \
                                    "$LOCALES_FILE")
            fi

            if [ -n "$KEY_LINE_IN_FILE" ]; then
                MISSING_IN_ZH_DETAILS="${MISSING_IN_ZH_DETAILS}\n  - 键 '$key' (en 定义于 $LOCALES_FILE:$KEY_LINE_IN_FILE) 在 'zh-CN' 中缺失。"
            else
                MISSING_IN_ZH_DETAILS="${MISSING_IN_ZH_DETAILS}\n  - 键 '$key' (en 中定义) 在 'zh-CN' 中缺失 (无法精确定位行号)。"
            fi
        fi
    done

    EXTRA_IN_ZH_DETAILS=""
    ZH_KEY_COUNT=0
    MATCHING_ZH_FOR_EN_KEYS=0

    for key in $ZH_KEYS; do
        ZH_KEY_COUNT=$((ZH_KEY_COUNT + 1))
        if ! echo "$EN_KEYS" | grep -Fxq "$key"; then
            ZH_SECTION_START_LINE=$(grep -nm1 "^zh-CN:" "$LOCALES_FILE" | cut -d: -f1)
            KEY_LINE_IN_FILE=""
            if [ -n "$ZH_SECTION_START_LINE" ]; then
                 KEY_LINE_IN_FILE=$(awk -v start_line="$ZH_SECTION_START_LINE" -v key_pattern="^[[:space:]]+${key}:" \
                                     'NR > start_line { if ($0 ~ key_pattern) { print NR; exit } if (!/^[[:space:]]/ && $0 !~ /^#/) { exit } }' \
                                     "$LOCALES_FILE")
            fi

            if [ -n "$KEY_LINE_IN_FILE" ]; then
                EXTRA_IN_ZH_DETAILS="${EXTRA_IN_ZH_DETAILS}\n  - 键 '$key' (zh-CN 定义于 $LOCALES_FILE:$KEY_LINE_IN_FILE) 存在于 'zh-CN' 但 'en' 中没有。"
            else
                EXTRA_IN_ZH_DETAILS="${EXTRA_IN_ZH_DETAILS}\n  - 键 '$key' (zh-CN 中定义) 存在于 'zh-CN' 但 'en' 中没有 (无法精确定位行号)。"
            fi
        else
            # This key from ZH_KEYS is also in EN_KEYS
            MATCHING_ZH_FOR_EN_KEYS=$((MATCHING_ZH_FOR_EN_KEYS + 1))
        fi
    done

    if [ -n "$MISSING_IN_ZH_DETAILS" ]; then
        echo "⚠️  'en' 中存在但 'zh-CN' 中缺失的翻译键:"
        echo -e "$MISSING_IN_ZH_DETAILS"
    else
        # Check if EN_KEY_COUNT is > 0 to avoid saying "all keys" if there are no keys.
        if [ "$EN_KEY_COUNT" -gt 0 ]; then
            echo "✅ 'zh-CN' 包含了所有 'en' 中的键。"
        else
            echo "ℹ️  'en' 部分没有发现任何键。"
        fi
    fi

    if [ -n "$EXTRA_IN_ZH_DETAILS" ]; then
        echo "⚠️  'zh-CN' 中存在但 'en' 中没有的额外翻译键:"
        echo -e "$EXTRA_IN_ZH_DETAILS"
    fi

    echo ""
    echo "📊 翻译键统计 ($LOCALES_FILE):"
    echo "  - 'en' 中的总键数: $EN_KEY_COUNT"
    echo "  - 'zh-CN' 中的总键数: $ZH_KEY_COUNT"
    # This counts keys that are in EN and also in ZH.
    # It's effectively EN_KEY_COUNT minus MISSING_IN_ZH_DETAILS count if we parse that.
    # A more direct calculation:
    ACTUALLY_MATCHED_EN_IN_ZH=0
    if [ -n "$EN_KEYS" ]; then # ensure EN_KEYS is not empty before looping
        for en_k_loop in $EN_KEYS; do
            if echo "$ZH_KEYS" | grep -Fxq "$en_k_loop"; then
                ACTUALLY_MATCHED_EN_IN_ZH=$((ACTUALLY_MATCHED_EN_IN_ZH + 1))
            fi
        done
    fi
    echo "  - 'en' 中的键在 'zh-CN' 中找到的数量: $ACTUALLY_MATCHED_EN_IN_ZH"


    if [ "$EN_KEY_COUNT" -eq 0 ] && [ "$ZH_KEY_COUNT" -eq 0 ]; then
        echo "ℹ️  翻译文件中 'en' 和 'zh-CN' 部分均未发现键。"
    elif [ "$EN_KEY_COUNT" -eq "$ACTUALLY_MATCHED_EN_IN_ZH" ] && [ "$ZH_KEY_COUNT" -eq "$ACTUALLY_MATCHED_EN_IN_ZH" ]; then
        echo "✅ 'en' 和 'zh-CN' 的键结构一致且完整。"
    elif [ "$EN_KEY_COUNT" -eq "$ACTUALLY_MATCHED_EN_IN_ZH" ] && [ "$ZH_KEY_COUNT" -gt "$ACTUALLY_MATCHED_EN_IN_ZH" ]; then
        echo "ℹ️  'zh-CN' 包含所有 'en' 键，但有一些额外的键。"
    elif [ "$EN_KEY_COUNT" -gt "$ACTUALLY_MATCHED_EN_IN_ZH" ]; then
        echo "⚠️  'zh-CN' 缺失部分 'en' 中的键。"
        if [ "$ZH_KEY_COUNT" -gt "$ACTUALLY_MATCHED_EN_IN_ZH" ]; then
             echo "   且 'zh-CN' 中也包含一些 'en' 中没有的额外键。"
        fi
    else
        # This case should ideally be covered by the above, but as a fallback:
        echo "ℹ️  键结构存在差异，请检查上述详细列表。"
    fi
fi

echo ""
echo "🎯 i18n 检查完成!"
echo ""
echo "💡 建议运行以下命令进行进一步测试:"
echo "   LLMAN_LANG=zh-CN just run prompt list"
echo "   LLMAN_LANG=en just run prompt list"
echo "   LLMAN_LANG=zh-CN just run prompt gen --app invalid --template test"

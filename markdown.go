package main

import (
	"fmt"
	"strings"
	"unicode"
)

// sourceLines matches Rust str::lines, including CRLF and trailing-newline handling.
func sourceLines(source string) []string {
	if source == "" {
		return nil
	}
	lines := strings.Split(source, "\n")
	if lines[len(lines)-1] == "" {
		lines = lines[:len(lines)-1]
	}
	for i := range lines {
		lines[i] = strings.TrimSuffix(lines[i], "\r")
	}
	return lines
}

func markdownToHTML(markdown string) string {
	lines := sourceLines(markdown)
	var html, paragraph, listItems, quote, codeLines []string
	listType, fence, language := "", "", ""
	flushParagraph := func() {
		if len(paragraph) > 0 {
			html = append(html, "<p>"+renderInline(strings.Join(paragraph, " "))+"</p>")
			paragraph = nil
		}
	}
	flushList := func() {
		if listType == "" {
			return
		}
		items := make([]string, 0, len(listItems))
		for _, item := range listItems {
			items = append(items, "  <li>"+renderInline(item)+"</li>")
		}
		html = append(html, "<"+listType+">\n"+strings.Join(items, "\n")+"\n</"+listType+">")
		listType, listItems = "", nil
	}
	flushQuote := func() {
		if len(quote) > 0 {
			html = append(html, "<blockquote>\n"+markdownToHTML(strings.Join(quote, "\n"))+"\n</blockquote>")
			quote = nil
		}
	}
	pushListItem := func(kind, item string) {
		if listType != "" && listType != kind {
			flushList()
		}
		listType = kind
		listItems = append(listItems, strings.TrimSpace(item))
	}
	for index := 0; index < len(lines); index++ {
		line := lines[index]
		trimmed := strings.TrimSpace(line)
		if fence != "" {
			if strings.HasPrefix(trimmed, fence) {
				html = append(html, renderCodeBlock(codeLines, language))
				fence, codeLines = "", nil
			} else {
				codeLines = append(codeLines, line)
			}
			continue
		}
		if trimmed == "" {
			flushParagraph()
			flushList()
			flushQuote()
			continue
		}
		if strings.HasPrefix(trimmed, "```") || strings.HasPrefix(trimmed, "~~~") {
			flushParagraph()
			flushList()
			flushQuote()
			fence, language = trimmed[:3], strings.TrimSpace(trimmed[3:])
			continue
		}
		if trimmed == "***" || (len(trimmed) >= 3 && strings.Trim(trimmed, "-") == "") {
			flushParagraph()
			flushList()
			flushQuote()
			html = append(html, "<hr>")
			continue
		}
		if rest, ok := strings.CutPrefix(line, "> "); ok {
			flushParagraph()
			flushList()
			quote = append(quote, rest)
			continue
		}
		if line == ">" {
			quote = append(quote, "")
			continue
		}
		flushQuote()
		hashes := len(line) - len(strings.TrimLeft(line, "#"))
		if hashes >= 1 && hashes <= 6 && len(line) > hashes && line[hashes] == ' ' {
			flushParagraph()
			flushList()
			content := strings.TrimSpace(line[hashes+1:])
			html = append(html, fmt.Sprintf("<h%d id=\"%s\">%s</h%d>", hashes, escapeAttr(headingID(content)), renderInline(content), hashes))
			continue
		}
		left := strings.TrimLeftFunc(line, unicode.IsSpace)
		if strings.HasPrefix(left, "- ") || strings.HasPrefix(left, "* ") {
			flushParagraph()
			pushListItem("ul", left[2:])
			continue
		}
		if item, ok := orderedItem(left); ok {
			flushParagraph()
			pushListItem("ol", item)
			continue
		}
		if strings.HasPrefix(line, "    ") {
			flushParagraph()
			flushList()
			flushQuote()
			code := []string{trimRepeatedPrefix(line, "    ")}
			for index+1 < len(lines) && strings.HasPrefix(lines[index+1], "    ") {
				index++
				code = append(code, trimRepeatedPrefix(lines[index], "    "))
			}
			html = append(html, renderCodeBlock(code, ""))
			continue
		}
		if index+1 < len(lines) {
			header, ok := tableCells(line)
			alignments, valid := tableAlignments(lines[index+1])
			if ok && valid && len(header) == len(alignments) {
				flushParagraph()
				flushList()
				var rows [][]string
				index++ // Consume the delimiter row.
				for index+1 < len(lines) {
					row, ok := tableCells(lines[index+1])
					if !ok {
						break
					}
					rows = append(rows, row)
					index++
				}
				html = append(html, renderTable(header, alignments, rows))
				continue
			}
		}
		flushList()
		paragraph = append(paragraph, trimmed)
	}
	flushParagraph()
	flushList()
	flushQuote()
	if fence != "" {
		html = append(html, renderCodeBlock(codeLines, ""))
	}
	return strings.Join(html, "\n")
}

// Pipe tables require a delimiter row; ordinary prose containing pipes stays prose.
func tableCells(line string) ([]string, bool) {
	line = strings.TrimSpace(line)
	var cells []string
	var cell strings.Builder
	lastDelimiter := false
	for i := 0; i < len(line); i++ {
		if line[i] == '\\' && i+1 < len(line) && (line[i+1] == '|' || line[i+1] == '\\') {
			cell.WriteByte(line[i+1])
			i++
			lastDelimiter = false
		} else if line[i] == '|' {
			cells = append(cells, strings.TrimSpace(cell.String()))
			cell.Reset()
			lastDelimiter = true
		} else {
			cell.WriteByte(line[i])
			lastDelimiter = false
		}
	}
	if len(cells) == 0 {
		return nil, false
	}
	cells = append(cells, strings.TrimSpace(cell.String()))
	if strings.HasPrefix(line, "|") {
		cells = cells[1:]
	}
	if lastDelimiter {
		cells = cells[:len(cells)-1]
	}
	return cells, len(cells) > 0
}

func tableAlignments(line string) ([]string, bool) {
	cells, ok := tableCells(line)
	if !ok {
		return nil, false
	}
	alignments := make([]string, len(cells))
	for i, cell := range cells {
		left, right := strings.HasPrefix(cell, ":"), strings.HasSuffix(cell, ":")
		dashes := strings.TrimSuffix(strings.TrimPrefix(cell, ":"), ":")
		if len(dashes) < 3 || strings.Trim(dashes, "-") != "" {
			return nil, false
		}
		switch {
		case left && right:
			alignments[i] = "center"
		case right:
			alignments[i] = "right"
		default:
			alignments[i] = "left"
		}
	}
	return alignments, true
}

func renderTable(header, alignments []string, rows [][]string) string {
	var output strings.Builder
	output.WriteString(`<div class="table-scroll" role="region" aria-label="表格，可横向滚动" tabindex="0"><table><thead><tr>`)
	for i, cell := range header {
		fmt.Fprintf(&output, `<th scope="col" style="text-align:%s">%s</th>`, alignments[i], renderInline(cell))
	}
	output.WriteString("</tr></thead><tbody>")
	for _, row := range rows {
		output.WriteString("<tr>")
		for i := range header {
			cell := ""
			if i < len(row) {
				cell = row[i]
			}
			fmt.Fprintf(&output, `<td style="text-align:%s">%s</td>`, alignments[i], renderInline(cell))
		}
		output.WriteString("</tr>")
	}
	output.WriteString("</tbody></table></div>")
	return output.String()
}

func trimRepeatedPrefix(value, prefix string) string {
	for strings.HasPrefix(value, prefix) {
		value = strings.TrimPrefix(value, prefix)
	}
	return value
}

func orderedItem(line string) (string, bool) {
	dot := strings.IndexByte(line, '.')
	if dot <= 0 {
		return "", false
	}
	for _, ch := range line[:dot] {
		if ch < '0' || ch > '9' {
			return "", false
		}
	}
	return strings.CutPrefix(line[dot+1:], " ")
}

func renderCodeBlock(lines []string, language string) string {
	class := ""
	if language != "" {
		class = " class=\"language-" + escapeAttr(language) + "\""
	}
	return "<pre><code" + class + ">" + escapeHTML(strings.Join(lines, "\n")) + "</code></pre>"
}

func renderInline(markdown string) string {
	var output strings.Builder
	rest := markdown
	for {
		start := strings.IndexByte(rest, '`')
		if start < 0 {
			break
		}
		output.WriteString(renderLinksAndImages(rest[:start]))
		after := rest[start+1:]
		end := strings.IndexByte(after, '`')
		if end >= 0 {
			output.WriteString("<code>" + escapeHTML(after[:end]) + "</code>")
			rest = after[end+1:]
		} else {
			output.WriteString(escapeHTML(rest[start:]))
			rest = ""
		}
	}
	output.WriteString(renderLinksAndImages(rest))
	return output.String()
}

func renderLinksAndImages(text string) string {
	var output strings.Builder
	rest := text
	for {
		open := strings.IndexByte(rest, '[')
		if open < 0 {
			break
		}
		image := open > 0 && rest[open-1] == '!'
		prefix := rest[:open]
		if image {
			prefix = prefix[:len(prefix)-1]
		}
		output.WriteString(escapeHTML(prefix))
		closeOffset := strings.IndexByte(rest[open+1:], ']')
		if closeOffset < 0 {
			output.WriteString(escapeHTML(rest[open:]))
			return output.String()
		}
		close := open + 1 + closeOffset
		after := rest[close+1:]
		if !strings.HasPrefix(after, "(") {
			output.WriteString(escapeHTML(rest[open : close+1]))
			rest = after
			continue
		}
		endOffset := strings.IndexByte(after[1:], ')')
		if endOffset < 0 {
			output.WriteString(escapeHTML(rest[open:]))
			return output.String()
		}
		end := endOffset + 1
		label := rest[open+1 : close]
		url := normalizeURL(after[1:end])
		if image {
			output.WriteString("<img src=\"" + escapeAttr(url) + "\" alt=\"" + escapeAttr(label) + "\">")
		} else {
			output.WriteString("<a href=\"" + escapeAttr(url) + "\">" + escapeHTML(label) + "</a>")
		}
		rest = after[end+1:]
	}
	output.WriteString(escapeHTML(rest))
	return output.String()
}

func normalizeURL(url string) string {
	trimmed := strings.Trim(strings.Trim(strings.TrimSpace(url), "\""), "'")
	for _, prefix := range []string{"http:", "https:", "mailto:", "#", "/"} {
		if strings.HasPrefix(trimmed, prefix) {
			return trimmed
		}
	}
	return "/" + trimRepeatedPrefix(trimmed, "./")
}

func headingID(markdown string) string {
	var text strings.Builder
	inLink := false
	for _, ch := range strings.ToLower(markdown) {
		if ch == '(' {
			break
		}
		switch ch {
		case '[':
			inLink = true
		case ']':
			inLink = false
		default:
			if inLink || unicode.IsLetter(ch) || unicode.IsNumber(ch) || unicode.IsSpace(ch) || ch == '-' || ch == '_' || ch == '.' {
				text.WriteRune(ch)
			}
		}
	}
	id := strings.Join(strings.Fields(text.String()), "-")
	if id == "" {
		return "section"
	}
	return id
}

func escapeHTML(value string) string {
	return strings.NewReplacer("&", "&amp;", "<", "&lt;", ">", "&gt;", "\"", "&quot;").Replace(value)
}
func escapeAttr(value string) string { return strings.ReplaceAll(escapeHTML(value), "'", "&#39;") }
func escapeXML(value string) string  { return strings.ReplaceAll(escapeHTML(value), "'", "&apos;") }

func encodeURI(value string) string {
	var output strings.Builder
	for _, b := range []byte(value) {
		if b >= 'a' && b <= 'z' || b >= 'A' && b <= 'Z' || b >= '0' && b <= '9' || strings.ContainsRune("-_.~/", rune(b)) {
			output.WriteByte(b)
		} else {
			fmt.Fprintf(&output, "%%%02X", b)
		}
	}
	return output.String()
}

func absoluteURL(config Config, route string) string {
	return strings.TrimRight(config.BaseURL, "/") + "/" + encodeURI(strings.TrimLeft(route, "/"))
}

func stripHTML(html string) string {
	var output strings.Builder
	inTag := false
	for _, ch := range html {
		switch ch {
		case '<':
			inTag = true
		case '>':
			inTag = false
			output.WriteByte(' ')
		default:
			if !inTag {
				output.WriteRune(ch)
			}
		}
	}
	return strings.Join(strings.Fields(output.String()), " ")
}

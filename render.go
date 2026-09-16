package main

import (
	"fmt"
	"strconv"
	"strings"
	"time"
)

const tcpThroughputRoute = "/labs/tcp-throughput/"

// Substitute all placeholders in one pass, so inserted content is never interpreted.
func renderTemplate(template string, pairs ...string) string {
	return strings.NewReplacer(pairs...).Replace(template)
}

func renderShell(config Config, title, canonicalPath, main string) string {
	pageTitle := title
	if title != config.Title {
		pageTitle += " | " + config.Title
	}
	return renderTemplate(shellTemplate, "{language}", escapeAttr(config.LanguageCode), "{page_title}", escapeHTML(pageTitle), "{canonical}", escapeAttr(absoluteURL(config, canonicalPath)), "{style}", strings.TrimSpace(style), "{site_title}", escapeHTML(config.Title), "{main}", main, "{year}", strconv.Itoa(time.Now().UTC().Year()))
}

func renderIndex(config Config, pages []Page) string {
	title, date := "", ""
	if len(pages) > 0 {
		title, date = pages[0].Title, pages[0].DateText
	}
	return renderTemplate(indexTemplate, "{language}", escapeAttr(config.LanguageCode), "{page_title}", escapeHTML(config.Title), "{canonical}", escapeAttr(absoluteURL(config, "/")), "{book_title}", escapeHTML(title), "{book_date}", escapeHTML(date), "{style}", strings.TrimSpace(homeStyle))
}

func postList(items []string) string {
	return "<ul class=\"post-list\">\n" + strings.Join(items, "\n") + "\n</ul>"
}

func renderPostIndex(config Config, pages []Page) string {
	main := "<p class=\"empty-state\">No posts yet.</p>"
	if len(pages) > 0 {
		var items []string
		for _, page := range pages {
			labEntry := ""
			if page.Route == "/posts/ping-28ms-single-tcp-post-throughput/" {
				labEntry = renderTemplate(labEntryTemplate, "{TCP_THROUGHPUT_ROUTE}", tcpThroughputRoute)
			}
			items = append(items, renderTemplate(postItemTemplate, "{href}", escapeAttr(encodeURI(page.Route)), "{title}", escapeHTML(page.Title), "{date}", escapeHTML(page.DateText), "{tag_links}", renderPostTags(page), "{lab_entry}", labEntry))
		}
		main = postList(items)
	}
	return renderShell(config, "Posts", "/posts/", main)
}

func renderTCPThroughputLab(config Config) string {
	return renderTemplate(labTemplate, "{site_title}", escapeHTML(config.Title), "{canonical}", escapeAttr(absoluteURL(config, tcpThroughputRoute)))
}

func renderPostTags(page Page) string {
	if len(page.Tags) == 0 {
		return ""
	}
	var links strings.Builder
	for _, tag := range page.Tags {
		fmt.Fprintf(&links, "<a href=\"%s\">#%s</a>", escapeAttr(encodeURI("/tags/"+slugify(tag)+"/")), escapeHTML(tag))
	}
	return "<div class=\"post-tags\">" + links.String() + "</div>"
}

func renderArticle(config Config, page Page) string {
	main := renderTemplate(articleTemplate, "{title}", escapeHTML(page.Title), "{date}", escapeHTML(page.DateText), "{tag_links}", renderPostTags(page), "{body}", page.HTML)
	return renderShell(config, page.Title, page.Route, main)
}

func renderTagIndex(config Config, tags []TagPage) string {
	var items []string
	for _, tag := range tags {
		items = append(items, renderTemplate(tagItemTemplate, "{href}", escapeAttr(encodeURI(tag.Route)), "{name}", escapeHTML(tag.Name), "{count}", strconv.Itoa(len(tag.Pages))))
	}
	return renderShell(config, "Tags", "/tags/", postList(items))
}

func renderTagPage(config Config, tag TagPage) string {
	var items []string
	for _, page := range tag.Pages {
		items = append(items, renderTemplate(tagPostTemplate, "{href}", escapeAttr(encodeURI(page.Route)), "{title}", escapeHTML(page.Title), "{date}", escapeHTML(page.DateText)))
	}
	return renderShell(config, "Tag: "+tag.Name, tag.Route, postList(items))
}

func renderFeed(config Config, pages []Page) string {
	var items []string
	for i, page := range pages {
		if i >= 20 {
			break
		}
		description := []rune(stripHTML(page.HTML))
		if len(description) > 500 {
			description = description[:500]
		}
		items = append(items, renderTemplate(feedItemTemplate, "{title}", escapeXML(page.Title), "{url}", escapeXML(absoluteURL(config, page.Route)), "{date}", escapeXML(page.DateText), "{description}", escapeXML(string(description))))
	}
	return renderTemplate(feedTemplate, "{title}", escapeXML(config.Title), "{base_url}", escapeXML(config.BaseURL), "{description}", escapeXML(config.Description), "{items}", strings.Join(items, "\n"))
}

func renderSitemap(config Config, pages []Page, tags []TagPage) string {
	routes := []string{"/", "/posts/", tcpThroughputRoute}
	for _, page := range pages {
		routes = append(routes, page.Route)
	}
	routes = append(routes, "/tags/")
	for _, tag := range tags {
		routes = append(routes, tag.Route)
	}
	var urls []string
	for _, route := range routes {
		urls = append(urls, "  <url>\n    <loc>"+escapeXML(absoluteURL(config, route))+"</loc>\n  </url>")
	}
	return renderTemplate(sitemapTemplate, "{urls}", strings.Join(urls, "\n"))
}

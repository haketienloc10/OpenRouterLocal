use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use serde_json::Value;

use crate::logging::db::{DashboardRequestRow, RequestListResult, RequestListSearch};

pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_status(err: &Option<String>) -> &'static str {
    if err.as_deref().is_some_and(|s| !s.trim().is_empty()) {
        "Error"
    } else {
        "OK"
    }
}

fn render_error_preview(err: &Option<String>) -> String {
    match err.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(text) => {
            let mut preview: String = text.chars().take(120).collect();
            if text.chars().count() > 120 {
                preview.push('…');
            }
            escape_html(&preview)
        }
        None => "-".to_string(),
    }
}

fn fmt_opt_i64(val: Option<i64>) -> String {
    val.map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_opt_f64(val: Option<f64>) -> String {
    val.map(|v| format!("{v:.6}"))
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_time_gmt7(created_at: i64) -> String {
    let secs = if created_at > 10_000_000_000 {
        created_at / 1000
    } else {
        created_at
    };

    let dt_utc: DateTime<Utc> = Utc.timestamp_opt(secs, 0).single().unwrap_or_else(|| {
        Utc.timestamp_opt(0, 0)
            .single()
            .expect("epoch must be valid")
    });
    let tz = FixedOffset::east_opt(7 * 3600).expect("GMT+7 offset should be valid");
    dt_utc
        .with_timezone(&tz)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn render_dashboard_page(
    models: &[String],
    providers: &[String],
    search: &RequestListSearch,
) -> String {
    let model_options = models
        .iter()
        .map(|m| {
            let selected = if search.model.as_deref() == Some(m.as_str()) {
                "selected"
            } else {
                ""
            };
            format!(
                "<option value=\"{}\" {}>{}</option>",
                escape_html(m),
                selected,
                escape_html(m)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let provider_options = providers
        .iter()
        .map(|p| {
            let selected = if search.provider.as_deref() == Some(p.as_str()) {
                "selected"
            } else {
                ""
            };
            format!(
                "<option value=\"{}\" {}>{}</option>",
                escape_html(p),
                selected,
                escape_html(p)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>OpenRouterLocal Dashboard</title>
  <script src="https://cdn.tailwindcss.com"></script>
  <script src="https://unpkg.com/htmx.org/dist/htmx.min.js"></script>
</head>
<body class="bg-slate-100 text-slate-900 min-h-screen">
  <main class="max-w-7xl mx-auto p-6 space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">OpenRouterLocal Dashboard</h1>
      <a href="/dashboard/errors" class="text-sm text-red-700 hover:underline">View error logs</a>
    </div>

    <form id="filters"
      class="bg-white rounded-lg shadow p-4 grid grid-cols-1 md:grid-cols-6 gap-3"
      hx-get="/dashboard/partials/requests"
      hx-target="#table-wrapper"
      hx-trigger="change, keyup changed delay:300ms from:#q"
      hx-swap="innerHTML">

      <input type="hidden" name="page" value="1" />

      <div>
        <label class="block text-sm font-medium">Model</label>
        <select name="model" class="w-full border rounded px-2 py-1">
          <option value="">All</option>
          {model_options}
        </select>
      </div>

      <div>
        <label class="block text-sm font-medium">Provider</label>
        <select name="provider" class="w-full border rounded px-2 py-1">
          <option value="">All</option>
          {provider_options}
        </select>
      </div>

      <div class="flex items-end">
        <label class="inline-flex items-center gap-2">
          <input type="checkbox" name="has_error" value="1" {checked} />
          <span class="text-sm">Only errors</span>
        </label>
      </div>

      <div>
        <label class="block text-sm font-medium">Search</label>
        <input id="q" name="q" value="{q}" placeholder="keyword" class="w-full border rounded px-2 py-1" />
      </div>

      <div>
        <label class="block text-sm font-medium">Page size</label>
        <select name="page_size" class="w-full border rounded px-2 py-1">
          <option value="20" {ps20}>20</option>
          <option value="50" {ps50}>50</option>
          <option value="100" {ps100}>100</option>
        </select>
      </div>
    </form>

    <div id="table-wrapper"
      class="bg-white rounded-lg shadow overflow-hidden"
      hx-get="/dashboard/partials/requests?page={page}&page_size={page_size}"
      hx-include="#filters"
      hx-trigger="load"
      hx-swap="innerHTML">
      <div class="p-4 text-sm text-slate-500">Loading...</div>
    </div>
  </main>
</body>
</html>"##,
        model_options = model_options,
        provider_options = provider_options,
        checked = if search.has_error { "checked" } else { "" },
        q = escape_html(search.q.as_deref().unwrap_or("")),
        ps20 = if search.page_size == 20 {
            "selected"
        } else {
            ""
        },
        ps50 = if search.page_size == 50 {
            "selected"
        } else {
            ""
        },
        ps100 = if search.page_size == 100 {
            "selected"
        } else {
            ""
        },
        page = search.page,
        page_size = search.page_size,
    )
}

pub fn render_dashboard_error_page(
    models: &[String],
    providers: &[String],
    search: &RequestListSearch,
) -> String {
    let mut forced = search.clone();
    forced.has_error = true;

    let page = render_dashboard_page(models, providers, &forced);
    page.replace("OpenRouterLocal Dashboard", "OpenRouterLocal Error Logs")
        .replace("Only errors", "Only errors (forced)")
        .replace(
            "name=\"has_error\" value=\"1\" checked",
            "name=\"has_error\" value=\"1\" checked disabled",
        )
}

pub fn render_requests_table(result: &RequestListResult, search: &RequestListSearch) -> String {
    let rows = if result.rows.is_empty() {
        "<tr><td class=\"p-3 text-center text-slate-500\" colspan=\"9\">No requests found</td></tr>"
            .to_string()
    } else {
        result
            .rows
            .iter()
            .map(render_request_row)
            .collect::<Vec<_>>()
            .join("")
    };

    let page_count = if result.total_count == 0 {
        1
    } else {
        ((result.total_count - 1) / i64::from(search.page_size) + 1) as i64
    };

    let prev_page = if search.page > 1 { search.page - 1 } else { 1 };
    let next_page = if (search.page as i64) < page_count {
        search.page + 1
    } else {
        search.page
    };

    format!(
        r##"<div>
<table class="w-full text-sm">
  <thead class="bg-slate-50 text-left">
    <tr>
      <th class="p-3">Time</th>
      <th class="p-3">Model</th>
      <th class="p-3">Provider</th>
      <th class="p-3">Tokens (p/c/t)</th>
      <th class="p-3">Latency (ms)</th>
      <th class="p-3">Cost</th>
      <th class="p-3">Status</th>
      <th class="p-3">Error</th>
      <th class="p-3">Action</th>
    </tr>
  </thead>
  <tbody id="req-rows" class="divide-y">
    {rows}
  </tbody>
</table>
<div class="p-3 flex items-center justify-between border-t bg-slate-50">
  <div class="text-xs text-slate-600">Total: {total_count} | Page {page} of {page_count}</div>
  <div class="flex gap-2">
    <button class="px-3 py-1 rounded border bg-white disabled:opacity-50" {prev_disabled}
      hx-get="/dashboard/partials/requests?page={prev_page}"
      hx-include="#filters"
      hx-target="#table-wrapper"
      hx-swap="innerHTML">Prev</button>
    <button class="px-3 py-1 rounded border bg-white disabled:opacity-50" {next_disabled}
      hx-get="/dashboard/partials/requests?page={next_page}"
      hx-include="#filters"
      hx-target="#table-wrapper"
      hx-swap="innerHTML">Next</button>
  </div>
</div>
</div>"##,
        rows = rows,
        total_count = result.total_count,
        page = search.page,
        page_count = page_count,
        prev_disabled = if search.page <= 1 { "disabled" } else { "" },
        next_disabled = if (search.page as i64) >= page_count {
            "disabled"
        } else {
            ""
        },
        prev_page = prev_page,
        next_page = next_page,
    )
}

fn render_request_row(row: &DashboardRequestRow) -> String {
    format!(
        r#"<tr>
<td class="p-3">{created_at}</td>
<td class="p-3">{model}</td>
<td class="p-3">{provider}</td>
<td class="p-3">{prompt}/{completion}/{total}</td>
<td class="p-3">{latency}</td>
<td class="p-3">{cost}</td>
<td class="p-3">{status}</td>
<td class="p-3 text-xs text-red-700">{error_preview}</td>
<td class="p-3"><a class="text-blue-600 hover:underline" href="/dashboard/requests/{id}">View</a></td>
</tr>"#,
        created_at = fmt_time_gmt7(row.created_at),
        model = escape_html(&row.model),
        provider = escape_html(&row.provider),
        prompt = fmt_opt_i64(row.prompt_tokens),
        completion = fmt_opt_i64(row.completion_tokens),
        total = fmt_opt_i64(row.total_tokens),
        latency = fmt_opt_i64(row.latency_ms),
        cost = fmt_opt_f64(row.cost),
        status = render_status(&row.error),
        error_preview = render_error_preview(&row.error),
        id = escape_html(&row.id),
    )
}

pub fn render_request_detail(row: &DashboardRequestRow) -> String {
    let parsed = serde_json::from_str::<Value>(&row.request_json)
        .map(|v| serde_json::to_string_pretty(&v).unwrap_or_else(|_| row.request_json.clone()))
        .unwrap_or_else(|_| row.request_json.clone());
    let response = row.response_text.clone().unwrap_or_default();

    format!(
        r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Request {id}</title>
  <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-slate-100 text-slate-900 min-h-screen">
  <main class="max-w-6xl mx-auto p-6 space-y-4">
    <a href="/dashboard" class="text-blue-600 hover:underline">← Back to dashboard</a>
    <h1 class="text-2xl font-bold">Request {id}</h1>

    <div class="bg-white rounded shadow p-4 grid grid-cols-1 md:grid-cols-3 gap-2 text-sm">
      <div><span class="font-semibold">Time:</span> {created_at}</div>
      <div><span class="font-semibold">Model:</span> {model}</div>
      <div><span class="font-semibold">Provider:</span> {provider}</div>
      <div><span class="font-semibold">Latency:</span> {latency} ms</div>
      <div><span class="font-semibold">Tokens:</span> {prompt}/{completion}/{total}</div>
      <div><span class="font-semibold">Cost:</span> {cost}</div>
      <div><span class="font-semibold">Status:</span> {status}</div>
    </div>

    <section class="bg-white rounded shadow p-4">
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-lg font-semibold">Request JSON</h2>
        <button class="px-2 py-1 border rounded text-sm" onclick="copyText('request-json')">Copy</button>
      </div>
      <pre id="request-json" class="bg-slate-50 p-3 rounded overflow-x-auto text-xs">{request_json}</pre>
    </section>

    <section class="bg-white rounded shadow p-4">
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-lg font-semibold">Response Text</h2>
        <button class="px-2 py-1 border rounded text-sm" onclick="copyText('response-text')">Copy</button>
      </div>
      <pre id="response-text" class="bg-slate-50 p-3 rounded overflow-x-auto text-xs whitespace-pre-wrap">{response}</pre>
    </section>

    {error_section}
  </main>
  <script>
    function copyText(id) {{
      const el = document.getElementById(id);
      if (!el) return;
      navigator.clipboard.writeText(el.textContent || '');
    }}
  </script>
</body>
</html>"##,
        id = escape_html(&row.id),
        created_at = fmt_time_gmt7(row.created_at),
        model = escape_html(&row.model),
        provider = escape_html(&row.provider),
        latency = fmt_opt_i64(row.latency_ms),
        prompt = fmt_opt_i64(row.prompt_tokens),
        completion = fmt_opt_i64(row.completion_tokens),
        total = fmt_opt_i64(row.total_tokens),
        cost = fmt_opt_f64(row.cost),
        status = render_status(&row.error),
        request_json = escape_html(&parsed),
        response = escape_html(&response),
        error_section = row.error.as_ref().map(|err| format!(
            "<section class=\"bg-red-50 border border-red-200 rounded shadow p-4\"><h2 class=\"text-lg font-semibold text-red-700\">Error</h2><pre class=\"mt-2 text-xs whitespace-pre-wrap\">{}</pre></section>",
            escape_html(err)
        )).unwrap_or_default(),
    )
}

pub fn render_not_found_page(id: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><script src=\"https://cdn.tailwindcss.com\"></script></head><body class=\"bg-slate-100\"><main class=\"max-w-xl mx-auto p-8\"><h1 class=\"text-2xl font-bold\">Request not found</h1><p class=\"mt-2\">No request found for id <code>{}</code>.</p><a class=\"text-blue-600 hover:underline\" href=\"/dashboard\">Back</a></main></body></html>",
        escape_html(id)
    )
}

pub fn render_error_page(message: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><script src=\"https://cdn.tailwindcss.com\"></script></head><body class=\"bg-slate-100\"><main class=\"max-w-xl mx-auto p-8\"><h1 class=\"text-2xl font-bold\">Internal server error</h1><p class=\"mt-2 text-red-700\">{}</p><a class=\"text-blue-600 hover:underline\" href=\"/dashboard\">Back</a></main></body></html>",
        escape_html(message)
    )
}

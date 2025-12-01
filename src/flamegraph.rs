//! Flame graph HTML generator.
//!
//! Takes collapsed stack data and renders an interactive HTML flame graph
//! with a modern dark theme.
//!
//! # Example
//!
//! ```
//! use std::collections::HashMap;
//! use flamegraph::generate_flamegraph;
//!
//! let mut stacks = HashMap::new();
//! stacks.insert("main;foo;bar".to_string(), 100);
//! stacks.insert("main;foo;baz".to_string(), 50);
//!
//! let html = generate_flamegraph(&stacks, "My Flame Graph", None);
//! std::fs::write("flamegraph.html", html).unwrap();
//! ```

use std::collections::HashMap;
use std::fmt::Write;

/// A frame in the flame graph.
#[derive(Debug, Clone)]
struct Frame {
    name: String,
    depth: usize,
    start: u64,
    end: u64,
}

/// Process stacks into frames using the flow/merge algorithm.
fn process_stacks(stacks: &HashMap<String, u64>) -> (Vec<Frame>, u64, usize) {
    let mut sorted: Vec<_> = stacks.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    let mut frames = Vec::new();
    let mut last_stack: Vec<&str> = Vec::new();
    let mut time: u64 = 0;
    let mut depth_max: usize = 0;
    let mut open_frames: HashMap<(String, usize), u64> = HashMap::new();

    for (stack_str, count) in &sorted {
        let this_stack: Vec<&str> = std::iter::once("")
            .chain(stack_str.split(';'))
            .collect();

        let len_same = last_stack
            .iter()
            .zip(this_stack.iter())
            .take_while(|(a, b)| a == b)
            .count();

        // Close frames no longer in path
        for i in (len_same..last_stack.len()).rev() {
            let name = last_stack[i].to_string();
            let key = (name.clone(), i);
            if let Some(start) = open_frames.remove(&key) {
                frames.push(Frame {
                    name,
                    depth: i,
                    start,
                    end: time,
                });
                depth_max = depth_max.max(i);
            }
        }

        // Open new frames
        for i in len_same..this_stack.len() {
            let name = this_stack[i].to_string();
            open_frames.insert((name, i), time);
        }

        time += *count;
        last_stack = this_stack;
    }

    // Close remaining frames
    for i in (0..last_stack.len()).rev() {
        let name = last_stack[i].to_string();
        let key = (name.clone(), i);
        if let Some(start) = open_frames.remove(&key) {
            frames.push(Frame {
                name,
                depth: i,
                start,
                end: time,
            });
            depth_max = depth_max.max(i);
        }
    }

    (frames, time, depth_max)
}

/// Generate a color for a function name (deterministic based on name hash).
fn color_for_name(name: &str) -> (u8, u8, u8) {
    if name.is_empty() {
        return (99, 102, 241); // Indigo for root
    }
    
    let hash: u32 = name.bytes().fold(0u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as u32)
    });
    
    // Generate hue from hash, keep saturation and lightness in good range for dark theme
    let hue = (hash % 360) as f64;
    let saturation = 0.65 + (((hash >> 8) % 20) as f64 / 100.0); // 0.65-0.85
    let lightness = 0.55 + (((hash >> 16) % 15) as f64 / 100.0); // 0.55-0.70
    
    hsl_to_rgb(hue, saturation, lightness)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    
    let (r, g, b) = match (h as u32) / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn format_samples(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

/// Generate a flame graph HTML document.
///
/// # Arguments
/// * `stacks` - HashMap of semicolon-separated stack traces to sample counts
/// * `title` - Title for the flame graph
/// * `subtitle` - Optional subtitle
///
/// # Returns
/// Complete HTML document as a string
pub fn generate_flamegraph(
    stacks: &HashMap<String, u64>,
    title: &str,
    subtitle: Option<&str>,
) -> String {
    let (frames, total_samples, depth_max) = process_stacks(stacks);
    
    if total_samples == 0 {
        return generate_error_html("No valid stack data provided");
    }

    let frame_height = 20;
    let chart_height = (depth_max + 1) * frame_height;

    let mut html = String::with_capacity(512 * 1024);
    
    // HTML header and styles
    write!(html, r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
* {{
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}}

body {{
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: linear-gradient(180deg, #0c0f1a 0%, #151928 100%);
    color: #e2e8f0;
    min-height: 100vh;
    overflow-x: hidden;
}}

.container {{
    max-width: 100%;
    padding: 24px;
}}

header {{
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 20px;
    flex-wrap: wrap;
    gap: 16px;
}}

.title-section h1 {{
    font-size: 1.75rem;
    font-weight: 600;
    color: #f1f5f9;
    letter-spacing: -0.025em;
    margin-bottom: 4px;
}}

.title-section .subtitle {{
    font-size: 0.875rem;
    color: #64748b;
    font-weight: 400;
}}

.controls {{
    display: flex;
    gap: 12px;
    align-items: center;
}}

.search-box {{
    position: relative;
}}

.search-box input {{
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 10px 16px 10px 40px;
    font-size: 0.875rem;
    color: #e2e8f0;
    width: 280px;
    transition: all 0.2s ease;
    outline: none;
}}

.search-box input:focus {{
    border-color: rgba(99, 102, 241, 0.5);
    background: rgba(255, 255, 255, 0.08);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
}}

.search-box input::placeholder {{
    color: #475569;
}}

.search-box svg {{
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: #475569;
    pointer-events: none;
}}

.btn {{
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 10px 16px;
    font-size: 0.875rem;
    color: #94a3b8;
    cursor: pointer;
    transition: all 0.2s ease;
    font-weight: 500;
}}

.btn:hover {{
    background: rgba(255, 255, 255, 0.1);
    color: #e2e8f0;
}}

.btn:disabled {{
    opacity: 0.5;
    cursor: not-allowed;
}}

.stats {{
    display: flex;
    gap: 24px;
    margin-bottom: 16px;
    flex-wrap: wrap;
}}

.stat {{
    display: flex;
    flex-direction: column;
    gap: 2px;
}}

.stat-label {{
    font-size: 0.75rem;
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}}

.stat-value {{
    font-size: 0.9375rem;
    color: #e2e8f0;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
}}

.chart-container {{
    position: relative;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    overflow: hidden;
}}

.chart {{
    position: relative;
    height: {chart_height}px;
    overflow: hidden;
}}

.frame {{
    position: absolute;
    height: {frame_height_css}px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    padding: 0 6px;
    font-size: 11px;
    font-family: 'SF Mono', 'Fira Code', 'JetBrains Mono', Consolas, monospace;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.9);
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    cursor: pointer;
    transition: filter 0.15s ease, transform 0.15s ease;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    border: 1px solid rgba(255, 255, 255, 0.1);
}}

.frame:hover {{
    filter: brightness(1.2);
    z-index: 100;
    border-color: rgba(255, 255, 255, 0.3);
}}

.frame.highlight {{
    background: rgb(250, 204, 21) !important;
    color: #1e1e1e !important;
    border-color: rgb(234, 179, 8) !important;
    text-shadow: none;
}}

.frame.faded {{
    opacity: 0.25;
}}

.frame.zoomed-parent {{
    opacity: 0.4;
}}

.frame.hidden {{
    display: none;
}}

.tooltip {{
    position: fixed;
    background: #1e293b;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 12px 16px;
    font-size: 0.8125rem;
    color: #e2e8f0;
    pointer-events: none;
    z-index: 1000;
    max-width: 500px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
    opacity: 0;
    transition: opacity 0.15s ease;
}}

.context-menu {{
    position: fixed;
    background: #1e293b;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    padding: 4px;
    font-size: 0.8125rem;
    color: #e2e8f0;
    z-index: 2000;
    min-width: 180px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255,255,255,0.05);
    display: none;
}}

.context-menu.visible {{
    display: block;
}}

.context-menu-item {{
    padding: 10px 12px;
    cursor: pointer;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 10px;
    transition: background 0.1s ease;
}}

.context-menu-item:hover {{
    background: rgba(255, 255, 255, 0.1);
}}

.context-menu-item svg {{
    width: 16px;
    height: 16px;
    opacity: 0.7;
}}

.context-menu-separator {{
    height: 1px;
    background: rgba(255, 255, 255, 0.1);
    margin: 4px 0;
}}

.tooltip.visible {{
    opacity: 1;
}}

.tooltip-name {{
    font-family: 'SF Mono', 'Fira Code', Consolas, monospace;
    font-weight: 600;
    color: #f1f5f9;
    margin-bottom: 8px;
    word-break: break-all;
}}

.tooltip-stats {{
    display: grid;
    grid-template-columns: auto auto;
    gap: 4px 16px;
    font-size: 0.75rem;
}}

.tooltip-stats dt {{
    color: #64748b;
}}

.tooltip-stats dd {{
    color: #94a3b8;
    font-variant-numeric: tabular-nums;
}}

footer {{
    margin-top: 16px;
    padding: 16px 0;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
}}

.footer-info {{
    font-size: 0.75rem;
    color: #475569;
}}

.keyboard-hints {{
    display: flex;
    gap: 16px;
    font-size: 0.75rem;
    color: #475569;
}}

.keyboard-hints kbd {{
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    padding: 2px 6px;
    font-family: inherit;
    font-size: 0.6875rem;
    margin-right: 4px;
}}

@media (max-width: 768px) {{
    .container {{
        padding: 16px;
    }}
    
    header {{
        flex-direction: column;
    }}
    
    .search-box input {{
        width: 100%;
    }}
    
    .controls {{
        width: 100%;
        flex-wrap: wrap;
    }}
}}
</style>
</head>
<body>
<div class="container">
    <header>
        <div class="title-section">
            <h1>{title_escaped}</h1>
            {subtitle_html}
        </div>
        <div class="controls">
            <div class="search-box">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="11" cy="11" r="8"/>
                    <path d="m21 21-4.35-4.35"/>
                </svg>
                <input type="text" id="search" placeholder="Search functions (regex)..." />
            </div>
            <button class="btn" id="resetZoom" disabled>Reset Zoom</button>
            <button class="btn" id="clearSearch" style="display:none">Clear Search</button>
        </div>
    </header>
    
    <div class="stats">
        <div class="stat">
            <span class="stat-label">Total Samples</span>
            <span class="stat-value">{total_samples_fmt}</span>
        </div>
        <div class="stat">
            <span class="stat-label">Max Depth</span>
            <span class="stat-value">{depth_max}</span>
        </div>
        <div class="stat" id="matchedStat" style="display:none">
            <span class="stat-label">Matched</span>
            <span class="stat-value" id="matchedValue">0%</span>
        </div>
    </div>
    
    <div class="chart-container">
        <div class="chart" id="chart">
"##,
        title = escape_html(title),
        chart_height = chart_height,
        frame_height_css = frame_height - 2,
        title_escaped = escape_html(title),
        subtitle_html = subtitle.map(|s| format!(r#"<p class="subtitle">{}</p>"#, escape_html(s))).unwrap_or_default(),
        total_samples_fmt = format_samples(total_samples),
        depth_max = depth_max
    ).unwrap();

    // Generate frames
    for frame in &frames {
        let duration = frame.end - frame.start;
        if duration == 0 {
            continue;
        }
        
        let width_pct = (duration as f64 / total_samples as f64) * 100.0;
        if width_pct < 0.08 {
            continue; // Skip very narrow frames
        }
        
        let left_pct = (frame.start as f64 / total_samples as f64) * 100.0;
        let bottom = frame.depth * frame_height;
        let pct = (duration as f64 / total_samples as f64) * 100.0;
        
        let (r, g, b) = color_for_name(&frame.name);
        let display_name = if frame.name.is_empty() { "all" } else { &frame.name };
        
        writeln!(
            html,
            r#"            <div class="frame" style="left:{:.4}%;width:{:.4}%;bottom:{}px;background:rgb({},{},{});" data-name="{}" data-samples="{}" data-pct="{:.2}" data-depth="{}" data-start="{}" data-end="{}">{}</div>"#,
            left_pct,
            width_pct,
            bottom,
            r, g, b,
            escape_html(display_name),
            duration,
            pct,
            frame.depth,
            frame.start,
            frame.end,
            escape_html(display_name)
        ).unwrap();
    }

    // Close chart and add tooltip + context menu + footer + script
    write!(html, r##"        </div>
    </div>
    
    <div class="tooltip" id="tooltip">
        <div class="tooltip-name" id="tooltipName"></div>
        <dl class="tooltip-stats">
            <dt>Samples</dt>
            <dd id="tooltipSamples"></dd>
            <dt>Percentage</dt>
            <dd id="tooltipPct"></dd>
            <dt>Self</dt>
            <dd id="tooltipSelf"></dd>
        </dl>
    </div>
    
    <div class="context-menu" id="contextMenu">
        <div class="context-menu-item" id="hideStack">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
                <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
                <path d="m1 1 22 22"/>
            </svg>
            <span>Hide this stack</span>
        </div>
        <div class="context-menu-separator"></div>
        <div class="context-menu-item" id="resetHidden">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M1 4v6h6"/>
                <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/>
            </svg>
            <span>Reset all hidden</span>
        </div>
    </div>
    
    <footer>
        <div class="footer-info">
            Generated by flamegraph-rs
        </div>
        <div class="keyboard-hints">
            <span><kbd>Click</kbd> Zoom in</span>
            <span><kbd>Right-click</kbd> Hide stack</span>
            <span><kbd>Esc</kbd> Reset</span>
            <span><kbd>/</kbd> Search</span>
        </div>
    </footer>
</div>

<script>
(function() {{
    const chart = document.getElementById('chart');
    const tooltip = document.getElementById('tooltip');
    const tooltipName = document.getElementById('tooltipName');
    const tooltipSamples = document.getElementById('tooltipSamples');
    const tooltipPct = document.getElementById('tooltipPct');
    const tooltipSelf = document.getElementById('tooltipSelf');
    const searchInput = document.getElementById('search');
    const resetBtn = document.getElementById('resetZoom');
    const clearSearchBtn = document.getElementById('clearSearch');
    const matchedStat = document.getElementById('matchedStat');
    const matchedValue = document.getElementById('matchedValue');
    const contextMenu = document.getElementById('contextMenu');
    const hideStackBtn = document.getElementById('hideStack');
    const resetHiddenBtn = document.getElementById('resetHidden');
    
    const frames = Array.from(document.querySelectorAll('.frame'));
    const totalSamples = {total_samples};
    
    let zoomedFrame = null;
    let searchTerm = null;
    let contextTarget = null;
    let hiddenStacks = new Set();
    
    // Store original positions
    frames.forEach(f => {{
        f.dataset.origStart = f.dataset.start;
        f.dataset.origEnd = f.dataset.end;
        f.dataset.origLeft = f.style.left;
        f.dataset.origWidth = f.style.width;
    }});
    
    function formatNumber(n) {{
        return n.toString().replace(/\B(?=(\d{{3}})+(?!\d))/g, ',');
    }}
    
    function isFrameHidden(frame) {{
        const start = parseInt(frame.dataset.origStart);
        const end = parseInt(frame.dataset.origEnd);
        const depth = parseInt(frame.dataset.depth);
        
        for (const hidden of hiddenStacks) {{
            if (start >= hidden.start && end <= hidden.end && depth >= hidden.depth) {{
                return true;
            }}
        }}
        return false;
    }}
    
    function recalculateLayout() {{
        // Determine visible frames
        const visibleFrames = frames.filter(f => !isFrameHidden(f));
        const hiddenFramesList = frames.filter(f => isFrameHidden(f));
        
        // Hide the hidden frames
        hiddenFramesList.forEach(f => f.classList.add('hidden'));
        
        // Group visible frames by depth
        const byDepth = new Map();
        visibleFrames.forEach(f => {{
            f.classList.remove('hidden');
            const depth = parseInt(f.dataset.depth);
            if (!byDepth.has(depth)) byDepth.set(depth, []);
            byDepth.get(depth).push(f);
        }});
        
        // Process depth 0 (root)
        const rootFrames = byDepth.get(0) || [];
        rootFrames.forEach(f => {{
            f.style.left = '0%';
            f.style.width = '100%';
        }});
        
        // Process each subsequent depth
        const maxDepth = Math.max(...visibleFrames.map(f => parseInt(f.dataset.depth)));
        
        for (let depth = 1; depth <= maxDepth; depth++) {{
            const framesAtDepth = byDepth.get(depth) || [];
            
            // Group by parent (frames at depth-1 that contain them)
            const parentGroups = new Map();
            
            framesAtDepth.forEach(f => {{
                const fStart = parseInt(f.dataset.origStart);
                const fEnd = parseInt(f.dataset.origEnd);
                
                // Find parent
                const parentFrames = byDepth.get(depth - 1) || [];
                const parent = parentFrames.find(p => {{
                    const pStart = parseInt(p.dataset.origStart);
                    const pEnd = parseInt(p.dataset.origEnd);
                    return pStart <= fStart && pEnd >= fEnd;
                }});
                
                if (parent) {{
                    const parentKey = parent.dataset.origStart + '-' + parent.dataset.origEnd;
                    if (!parentGroups.has(parentKey)) {{
                        parentGroups.set(parentKey, {{ parent, children: [] }});
                    }}
                    parentGroups.get(parentKey).children.push(f);
                }}
            }});
            
            // Position each group's children within their parent
            parentGroups.forEach(({{ parent, children }}) => {{
                const parentLeft = parseFloat(parent.style.left);
                const parentWidth = parseFloat(parent.style.width);
                
                // Sort children by original start position
                children.sort((a, b) => parseInt(a.dataset.origStart) - parseInt(b.dataset.origStart));
                
                // Calculate total samples of children
                const totalChildSamples = children.reduce((sum, c) => {{
                    return sum + (parseInt(c.dataset.origEnd) - parseInt(c.dataset.origStart));
                }}, 0);
                
                // Position children proportionally
                let currentLeft = parentLeft;
                children.forEach(child => {{
                    const childSamples = parseInt(child.dataset.origEnd) - parseInt(child.dataset.origStart);
                    const widthPct = (childSamples / totalChildSamples) * parentWidth;
                    
                    child.style.left = currentLeft + '%';
                    child.style.width = widthPct + '%';
                    
                    currentLeft += widthPct;
                }});
            }});
        }}
        
        applySearch();
    }}
    
    // Tooltip handling
    frames.forEach(frame => {{
        frame.addEventListener('mouseenter', (e) => {{
            const name = frame.dataset.name;
            const samples = parseInt(frame.dataset.samples);
            const pct = parseFloat(frame.dataset.pct);
            
            const depth = parseInt(frame.dataset.depth);
            const start = parseInt(frame.dataset.start);
            const end = parseInt(frame.dataset.end);
            
            let childSamples = 0;
            frames.forEach(f => {{
                if (f.classList.contains('hidden')) return;
                const fDepth = parseInt(f.dataset.depth);
                const fStart = parseInt(f.dataset.start);
                const fEnd = parseInt(f.dataset.end);
                if (fDepth === depth + 1 && fStart >= start && fEnd <= end) {{
                    childSamples += parseInt(f.dataset.samples);
                }}
            }});
            
            const selfSamples = samples - childSamples;
            const selfPct = (selfSamples / totalSamples * 100).toFixed(2);
            
            tooltipName.textContent = name;
            tooltipSamples.textContent = formatNumber(samples);
            tooltipPct.textContent = pct.toFixed(2) + '%';
            tooltipSelf.textContent = formatNumber(selfSamples) + ' (' + selfPct + '%)';
            tooltip.classList.add('visible');
        }});
        
        frame.addEventListener('mouseleave', () => {{
            tooltip.classList.remove('visible');
        }});
        
        frame.addEventListener('mousemove', (e) => {{
            const x = e.clientX + 16;
            const y = e.clientY + 16;
            const rect = tooltip.getBoundingClientRect();
            const maxX = window.innerWidth - rect.width - 16;
            const maxY = window.innerHeight - rect.height - 16;
            tooltip.style.left = Math.min(x, maxX) + 'px';
            tooltip.style.top = Math.min(y, maxY) + 'px';
        }});
        
        frame.addEventListener('click', () => {{
            zoomTo(frame);
        }});
        
        frame.addEventListener('contextmenu', (e) => {{
            e.preventDefault();
            contextTarget = frame;
            
            contextMenu.style.left = e.clientX + 'px';
            contextMenu.style.top = e.clientY + 'px';
            contextMenu.classList.add('visible');
            
            setTimeout(() => {{
                const rect = contextMenu.getBoundingClientRect();
                if (rect.right > window.innerWidth) {{
                    contextMenu.style.left = (e.clientX - rect.width) + 'px';
                }}
                if (rect.bottom > window.innerHeight) {{
                    contextMenu.style.top = (e.clientY - rect.height) + 'px';
                }}
            }}, 0);
        }});
    }});
    
    function hideContextMenu() {{
        contextMenu.classList.remove('visible');
        contextTarget = null;
    }}
    
    hideStackBtn.addEventListener('click', () => {{
        if (!contextTarget) return;
        
        const start = parseInt(contextTarget.dataset.origStart);
        const end = parseInt(contextTarget.dataset.origEnd);
        const depth = parseInt(contextTarget.dataset.depth);
        
        hiddenStacks.add({{ start, end, depth }});
        hideContextMenu();
        recalculateLayout();
        resetBtn.disabled = false;
    }});
    
    resetHiddenBtn.addEventListener('click', () => {{
        hideContextMenu();
        resetAll();
    }});
    
    function zoomTo(frame) {{
        if (!frame || frame.classList.contains('hidden')) return;
        
        const targetStart = parseInt(frame.dataset.start);
        const targetEnd = parseInt(frame.dataset.end);
        const targetDepth = parseInt(frame.dataset.depth);
        const targetWidth = targetEnd - targetStart;
        
        zoomedFrame = frame;
        resetBtn.disabled = false;
        
        frames.forEach(f => {{
            if (f.classList.contains('hidden')) return;
            
            const fStart = parseInt(f.dataset.start);
            const fEnd = parseInt(f.dataset.end);
            const fDepth = parseInt(f.dataset.depth);
            
            f.classList.remove('zoomed-parent', 'faded');
            
            if (fEnd <= targetStart || fStart >= targetEnd) {{
                f.classList.add('hidden');
                return;
            }}
            
            if (fDepth < targetDepth && fStart <= targetStart && fEnd >= targetEnd) {{
                f.classList.add('zoomed-parent');
                f.style.left = '0%';
                f.style.width = '100%';
                return;
            }}
            
            const newStart = Math.max(0, fStart - targetStart);
            const newEnd = Math.min(targetWidth, fEnd - targetStart);
            const newWidth = newEnd - newStart;
            
            const leftPct = (newStart / targetWidth) * 100;
            const widthPct = (newWidth / targetWidth) * 100;
            
            f.style.left = leftPct + '%';
            f.style.width = widthPct + '%';
        }});
        
        applySearch();
    }}
    
    function resetAll() {{
        zoomedFrame = null;
        hiddenStacks.clear();
        resetBtn.disabled = true;
        searchTerm = null;
        searchInput.value = '';
        
        frames.forEach(f => {{
            f.classList.remove('hidden', 'zoomed-parent', 'faded', 'highlight');
            f.style.left = f.dataset.origLeft;
            f.style.width = f.dataset.origWidth;
        }});
        
        matchedStat.style.display = 'none';
        clearSearchBtn.style.display = 'none';
    }}
    
    function applySearch() {{
        if (!searchTerm) {{
            frames.forEach(f => {{
                if (!f.classList.contains('hidden')) {{
                    f.classList.remove('highlight', 'faded');
                }}
            }});
            matchedStat.style.display = 'none';
            clearSearchBtn.style.display = 'none';
            return;
        }}
        
        let regex;
        try {{
            regex = new RegExp(searchTerm, 'i');
        }} catch (e) {{
            return;
        }}
        
        let matchedSamples = 0;
        let visibleSamples = 0;
        
        frames.forEach(f => {{
            if (f.classList.contains('hidden')) return;
            
            const samples = parseInt(f.dataset.samples);
            const name = f.dataset.name;
            
            if (!f.classList.contains('zoomed-parent')) {{
                visibleSamples = Math.max(visibleSamples, samples);
            }}
            
            if (regex.test(name)) {{
                f.classList.add('highlight');
                f.classList.remove('faded');
                matchedSamples += samples;
            }} else {{
                f.classList.remove('highlight');
                f.classList.add('faded');
            }}
        }});
        
        const matchedPct = visibleSamples > 0 ? (matchedSamples / visibleSamples * 100) : 0;
        matchedValue.textContent = matchedPct.toFixed(1) + '%';
        matchedStat.style.display = 'flex';
        clearSearchBtn.style.display = 'block';
    }}
    
    function clearSearch() {{
        searchTerm = null;
        searchInput.value = '';
        applySearch();
        if (hiddenStacks.size === 0 && !zoomedFrame) {{
            resetBtn.disabled = true;
        }}
    }}
    
    // Event listeners
    document.addEventListener('click', (e) => {{
        if (!contextMenu.contains(e.target) && !e.target.closest('.frame')) {{
            hideContextMenu();
        }}
    }});
    
    searchInput.addEventListener('input', (e) => {{
        searchTerm = e.target.value || null;
        applySearch();
        if (searchTerm) resetBtn.disabled = false;
    }});
    
    resetBtn.addEventListener('click', resetAll);
    clearSearchBtn.addEventListener('click', clearSearch);
    
    document.addEventListener('keydown', (e) => {{
        if (e.key === 'Escape') {{
            if (contextMenu.classList.contains('visible')) {{
                hideContextMenu();
            }} else if (searchTerm || hiddenStacks.size > 0 || zoomedFrame) {{
                resetAll();
            }}
        }} else if (e.key === '/' && document.activeElement !== searchInput) {{
            e.preventDefault();
            searchInput.focus();
        }}
    }});
}})();
</script>
</body>
</html>"##,
        total_samples = total_samples
    ).unwrap();

    html
}

fn generate_error_html(message: &str) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Error</title>
<style>
body {{
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #0c0f1a;
    color: #e2e8f0;
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
}}
.error {{
    text-align: center;
    padding: 48px;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 12px;
}}
.error h1 {{
    color: #f87171;
    font-size: 1.25rem;
    margin-bottom: 8px;
}}
.error p {{
    color: #94a3b8;
}}
</style>
</head>
<body>
<div class="error">
    <h1>Error</h1>
    <p>{}</p>
</div>
</body>
</html>"#, escape_html(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_generation() {
        let mut stacks = HashMap::new();
        stacks.insert("main;foo;bar".to_string(), 100);
        stacks.insert("main;foo;baz".to_string(), 50);
        stacks.insert("main;qux".to_string(), 25);

        let html = generate_flamegraph(&stacks, "Test Graph", None);
        
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Test Graph"));
        assert!(html.contains("main"));
        assert!(html.contains("foo"));
        assert!(html.contains("bar"));
    }

    #[test]
    fn test_with_subtitle() {
        let mut stacks = HashMap::new();
        stacks.insert("a;b".to_string(), 10);

        let html = generate_flamegraph(&stacks, "Title", Some("My Subtitle"));
        
        assert!(html.contains("My Subtitle"));
    }

    #[test]
    fn test_empty_stacks() {
        let stacks = HashMap::new();
        let html = generate_flamegraph(&stacks, "Empty", None);
        
        assert!(html.contains("Error"));
        assert!(html.contains("No valid stack data"));
    }

    #[test]
    fn test_html_escaping() {
        let mut stacks = HashMap::new();
        stacks.insert("main;<script>alert('xss')</script>".to_string(), 10);

        let html = generate_flamegraph(&stacks, "Test <XSS>", None);
        
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_color_generation() {
        let (r1, g1, b1) = color_for_name("function_a");
        let (r2, g2, b2) = color_for_name("function_a");
        
        // Same name should produce same color
        assert_eq!((r1, g1, b1), (r2, g2, b2));
    }

    #[test]
    fn test_format_samples() {
        assert_eq!(format_samples(1), "1");
        assert_eq!(format_samples(1000), "1,000");
        assert_eq!(format_samples(1000000), "1,000,000");
    }
}
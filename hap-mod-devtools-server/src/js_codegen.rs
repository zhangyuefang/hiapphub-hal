use serde_json::Value;

fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{}\"", s))
}

pub fn dom_query(selector: &str, query_type: &str, all: bool, limit: usize, include_styles: bool) -> String {
    let sel_json = json_str(selector);
    let style_code = if include_styles {
        "var cs=window.getComputedStyle(el);o.computedStyle={display:cs.display,visibility:cs.visibility,opacity:cs.opacity,color:cs.color,backgroundColor:cs.backgroundColor,fontSize:cs.fontSize,position:cs.position};"
    } else {
        ""
    };
    let serialize = format!(
        "function __s(el){{var r=el.getBoundingClientRect();var attrs={{}};for(var i=0;i<el.attributes.length;i++){{var a=el.attributes[i];attrs[a.name]=a.value}}var o={{tagName:el.tagName.toLowerCase(),className:el.className||'',textContent:(el.textContent||'').slice(0,200),boundingRect:{{x:Math.round(r.x),y:Math.round(r.y),width:Math.round(r.width),height:Math.round(r.height)}},attributes:attrs}};{style_code}return o}}"
    );

    let resolved_type = if query_type == "auto" {
        if selector.starts_with('/') || selector.starts_with('(') {
            "xpath"
        } else if selector.starts_with("text=") {
            "text"
        } else {
            "css"
        }
    } else {
        query_type
    };

    match resolved_type {
        "xpath" => {
            if all {
                format!("(function(){{{serialize};var it=document.evaluate({sel_json},document,null,XPathResult.ORDERED_NODE_SNAPSHOT_TYPE,null);var res=[];for(var i=0;i<Math.min(it.snapshotLength,{limit});i++){{res.push(__s(it.snapshotItem(i)))}}return{{count:it.snapshotLength,elements:res}}}})()") 
            } else {
                format!("(function(){{{serialize};var it=document.evaluate({sel_json},document,null,XPathResult.FIRST_ORDERED_NODE_TYPE,null);var el=it.singleNodeValue;if(!el)return{{found:false}};return{{found:true,element:__s(el)}}}})()") 
            }
        }
        "text" => {
            let text = if let Some(t) = selector.strip_prefix("text=") { t } else { selector };
            let text_json = json_str(text);
            let walk = format!("function __walk(node,acc,lim){{if(acc.length>=lim)return;if(node.nodeType===1){{if((node.textContent||'').includes({text_json})){{var direct=false;for(var c=node.childNodes,i=0;i<c.length;i++){{if(c[i].nodeType===3&&c[i].textContent.includes({text_json})){{direct=true;break}}}}if(direct)acc.push(node)}}}}for(var ch=node.children,j=0;j<(ch?ch.length:0);j++){{__walk(ch[j],acc,lim)}}}}");
            if all {
                format!("(function(){{{serialize};{walk};var acc=[];__walk(document.body,acc,{limit});return{{count:acc.length,elements:acc.map(__s)}}}})()") 
            } else {
                format!("(function(){{{serialize};{walk};var acc=[];__walk(document.body,acc,1);if(!acc.length)return{{found:false}};return{{found:true,element:__s(acc[0])}}}})()") 
            }
        }
        _ => {
            if all {
                format!("(function(){{{serialize};var els=document.querySelectorAll({sel_json});var res=[];for(var i=0;i<Math.min(els.length,{limit});i++){{res.push(__s(els[i]))}}return{{count:els.length,elements:res}}}})()") 
            } else {
                format!("(function(){{{serialize};var el=document.querySelector({sel_json});if(!el)return{{found:false}};return{{found:true,element:__s(el)}}}})()") 
            }
        }
    }
}

pub fn dom_tree(selector: Option<&str>, max_depth: u32) -> String {
    let root_sel = selector.map(json_str).unwrap_or_else(|| "null".into());
    format!(
        "(function(){{var md={max_depth};function ser(el,d){{if(!el||d>md&&md!==-1)return null;var o={{tag:el.tagName?el.tagName.toLowerCase():'#text'}};if(!el.tagName){{o.text=el.textContent;return o}}if(el.id)o.id=el.id;if(el.className&&typeof el.className==='string')o.class=el.className;var ch=[];for(var i=0;i<el.childNodes.length;i++){{var c=ser(el.childNodes[i],d+1);if(c)ch.push(c)}}if(ch.length)o.children=ch;return o}}var root={root_sel}?document.querySelector({root_sel}):document.body;if(!root)return{{error:'root not found'}};return ser(root,0)}})()"
    )
}

pub fn snapshot(selector: &str, max_depth: u32) -> String {
    let sel_json = json_str(selector);
    format!(
        "(function(){{var root=document.querySelector({sel_json});if(!root)return{{error:'selector not found'}};function walk(el,d){{if(d>{max_depth})return{{tag:el.tagName.toLowerCase(),truncated:true}};var o={{tag:el.tagName.toLowerCase()}};if(el.id)o.id=el.id;if(el.className)o.cls=el.className;var t='';for(var i=0;i<el.childNodes.length;i++){{if(el.childNodes[i].nodeType===3)t+=el.childNodes[i].textContent}}t=t.trim();if(t)o.text=t.slice(0,100);var ch=[];for(var j=0;j<el.children.length;j++){{ch.push(walk(el.children[j],d+1))}}if(ch.length)o.children=ch;return o}}return walk(root,0)}})()"
    )
}

pub fn click(body: &Value) -> String {
    if let Some(sel) = body.get("selector").and_then(|s| s.as_str()) {
        let sel_json = json_str(sel);
        format!("(function(){{var el=document.querySelector({sel_json});if(!el)return {{error:'element not found'}};el.click();return {{success:true}}}})()")
    } else if let (Some(x), Some(y)) = (body.get("x").and_then(|v| v.as_f64()), body.get("y").and_then(|v| v.as_f64())) {
        format!("(function(){{var el=document.elementFromPoint({x},{y});if(!el)return {{error:'no element at coordinates'}};el.click();return {{success:true}}}})()")
    } else {
        "({error:'selector or x/y required'})".into()
    }
}

pub fn type_text(body: &Value) -> String {
    let text = body.get("text").and_then(|t| t.as_str()).unwrap_or("");
    let text_json = json_str(text);
    if let Some(sel) = body.get("selector").and_then(|s| s.as_str()) {
        let sel_json = json_str(sel);
        format!("(function(){{var el=document.querySelector({sel_json});if(!el)return {{error:'element not found'}};el.focus();document.execCommand('insertText',false,{text_json});return {{success:true}}}})()")
    } else {
        format!("(function(){{var el=document.activeElement;if(!el)return {{error:'no focused element'}};document.execCommand('insertText',false,{text_json});return {{success:true}}}})()")
    }
}

pub fn scroll(body: &Value) -> String {
    let x = body.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let y = body.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if let Some(sel) = body.get("selector").and_then(|s| s.as_str()) {
        let sel_json = json_str(sel);
        format!("(function(){{var el=document.querySelector({sel_json});if(!el)return {{error:'element not found'}};el.scrollBy({x},{y});return {{success:true}}}})()")
    } else {
        format!("(function(){{window.scrollBy({x},{y});return {{success:true}}}})()")
    }
}

pub fn wait_for_selector(selector: &str, timeout: u64) -> String {
    let sel_json = json_str(selector);
    format!("(async function(){{var s={sel_json},t={timeout},i=50,e=0;while(e<t){{var el=document.querySelector(s);if(el)return{{found:true,tag:el.tagName.toLowerCase()}};await new Promise(r=>setTimeout(r,i));e+=i;}}return{{found:false,timeout:true}}}})()")
}

pub fn wait_for_navigation(timeout: u64) -> String {
    format!("(async function(){{var h=location.href,t={timeout},i=100,e=0;while(e<t){{if(location.href!==h)return{{navigated:true,url:location.href}};await new Promise(r=>setTimeout(r,i));e+=i;}}return{{navigated:false,timeout:true}}}})()")
}

pub fn wait_for_idle(timeout: u64) -> String {
    format!("(async function(){{return new Promise(function(resolve){{var t=setTimeout(function(){{resolve({{idle:false,timeout:true}})}},{timeout});var ric=window.requestIdleCallback||function(cb){{setTimeout(cb,50)}};ric(function(){{clearTimeout(t);resolve({{idle:true}})}},{{timeout:{timeout}}})}})}})()")
}

pub fn console_start() -> String {
    "(function(){if(window.__hapConsoleLog)return{already:true};window.__hapConsoleLog=[];var o=window.console;var wrap=function(level){var orig=o[level];o[level]=function(){var a=Array.prototype.slice.call(arguments).map(function(x){try{return typeof x==='object'?JSON.stringify(x):String(x)}catch(e){return String(x)}});window.__hapConsoleLog.push({level:level,args:a,time:Date.now()});if(window.__hapConsoleLog.length>500)window.__hapConsoleLog.shift();orig.apply(o,arguments)}};['log','warn','error','info','debug'].forEach(wrap);return{started:true}})()".into()
}

pub fn console_logs(since: u64, level: &str) -> String {
    let level_json = json_str(level);
    format!("(function(){{var logs=window.__hapConsoleLog||[];var s={since};var l={level_json};var f=logs.filter(function(e){{return e.time>s&&(!l||e.level===l)}});return{{logs:f.slice(-100),total:f.length}}}})()")
}

pub fn dom_observe(selector: &str, child_list: bool, attributes: bool, subtree: bool, character_data: bool) -> String {
    let sel_json = json_str(selector);
    format!(
        "(function(){{if(window.__hapMutObs){{window.__hapMutObs.disconnect()}}window.__hapMutLog=[];var tgt=document.querySelector({sel_json});if(!tgt)return{{error:'selector not found'}};window.__hapMutObs=new MutationObserver(function(muts){{muts.forEach(function(m){{var e={{type:m.type,target:m.target.tagName?m.target.tagName.toLowerCase():'#text'}};if(m.type==='attributes')e.attributeName=m.attributeName;if(m.type==='childList'){{e.addedNodes=m.addedNodes.length;e.removedNodes=m.removedNodes.length}}if(m.type==='characterData')e.oldValue=(m.oldValue||'').slice(0,100);e.time=Date.now();window.__hapMutLog.push(e);if(window.__hapMutLog.length>200)window.__hapMutLog.shift()}})}});window.__hapMutObs.observe(tgt,{{childList:{child_list},attributes:{attributes},subtree:{subtree},characterData:{character_data}}});return{{observing:true,selector:{sel_json}}}}})()"
    )
}

pub fn dom_mutations(since: u64, clear: bool) -> String {
    let clear_code = if clear { "window.__hapMutLog=[];" } else { "" };
    format!("(function(){{var log=window.__hapMutLog||[];var s={since};var f=log.filter(function(e){{return e.time>s}});{clear_code}return{{mutations:f.slice(-100),total:f.length}}}})()")
}

pub fn dom_observe_stop() -> String {
    "(function(){if(window.__hapMutObs){window.__hapMutObs.disconnect();window.__hapMutObs=null;return{stopped:true}}return{stopped:false,reason:'no active observer'}})()".into()
}

pub fn accessibility(selector: &str, max_depth: u32) -> String {
    let sel_json = json_str(selector);
    format!(
        "(function(){{var root=document.querySelector({sel_json});if(!root)return{{error:'selector not found'}};function walk(el,d){{if(d>{max_depth})return{{role:el.getAttribute('role')||el.tagName.toLowerCase(),truncated:true}};var o={{role:el.getAttribute('role')||el.tagName.toLowerCase()}};var n=el.getAttribute('aria-label')||el.getAttribute('alt')||el.getAttribute('title');if(n)o.name=n;var s={{}};if(el.getAttribute('aria-hidden'))s.hidden=el.getAttribute('aria-hidden')==='true';if(el.getAttribute('aria-disabled'))s.disabled=el.getAttribute('aria-disabled')==='true';if(el.getAttribute('aria-expanded'))s.expanded=el.getAttribute('aria-expanded')==='true';if(el.getAttribute('aria-checked'))s.checked=el.getAttribute('aria-checked');if(el.getAttribute('aria-selected'))s.selected=el.getAttribute('aria-selected')==='true';if(Object.keys(s).length)o.states=s;if(el.getAttribute('tabindex'))o.focusable=true;var t='';for(var i=0;i<el.childNodes.length;i++){{if(el.childNodes[i].nodeType===3)t+=el.childNodes[i].textContent}}t=t.trim();if(t&&!o.name)o.name=t.slice(0,100);var ch=[];for(var j=0;j<el.children.length;j++){{ch.push(walk(el.children[j],d+1))}}if(ch.length)o.children=ch;return o}}return walk(root,0)}})()"
    )
}

pub fn performance() -> String {
    "(function(){var r={};if(window.performance){var nav=performance.getEntriesByType('navigation');if(nav&&nav[0]){var n=nav[0];r.navigation={domContentLoaded:Math.round(n.domContentLoadedEventEnd-n.startTime),load:Math.round(n.loadEventEnd-n.startTime),domInteractive:Math.round(n.domInteractive-n.startTime),responseEnd:Math.round(n.responseEnd-n.startTime)}}var paint=performance.getEntriesByType('paint');if(paint){paint.forEach(function(p){if(p.name==='first-contentful-paint')r.fcp=Math.round(p.startTime);if(p.name==='first-paint')r.fp=Math.round(p.startTime)})}if(performance.memory){r.memory={usedJSHeapSize:performance.memory.usedJSHeapSize,totalJSHeapSize:performance.memory.totalJSHeapSize,jsHeapSizeLimit:performance.memory.jsHeapSizeLimit}}r.resourceCount=performance.getEntriesByType('resource').length;r.now=Math.round(performance.now())}return r})()".into()
}

pub fn network_start() -> String {
    "(function(){if(window.__hapNetLog)return{already:true};window.__hapNetLog=[];var oF=window.fetch;window.__hapNetOrigFetch=oF;window.fetch=function(){var url=arguments[0];var opts=arguments[1]||{};var entry={type:'fetch',method:(opts.method||'GET').toUpperCase(),url:typeof url==='string'?url:url.url,startTime:Date.now(),status:0};window.__hapNetLog.push(entry);if(window.__hapNetLog.length>300)window.__hapNetLog.shift();return oF.apply(this,arguments).then(function(res){entry.status=res.status;entry.duration=Date.now()-entry.startTime;return res}).catch(function(err){entry.error=err.message;entry.duration=Date.now()-entry.startTime;throw err})};var oX=window.XMLHttpRequest;window.__hapNetOrigXHR=oX;window.XMLHttpRequest=function(){var x=new oX();var entry={type:'xhr',method:'',url:'',startTime:0,status:0};var oOpen=x.open;x.open=function(m,u){entry.method=m;entry.url=u;oOpen.apply(x,arguments)};var oSend=x.send;x.send=function(){entry.startTime=Date.now();window.__hapNetLog.push(entry);if(window.__hapNetLog.length>300)window.__hapNetLog.shift();oSend.apply(x,arguments)};x.addEventListener('loadend',function(){entry.status=x.status;entry.duration=Date.now()-entry.startTime});return x};return{started:true}})()".into()
}

pub fn network_requests(since: u64, clear: bool) -> String {
    let clear_code = if clear { "window.__hapNetLog=[];" } else { "" };
    format!("(function(){{var log=window.__hapNetLog||[];var s={since};var f=log.filter(function(e){{return e.startTime>s}});{clear_code}return{{requests:f.slice(-100),total:f.length}}}})()")
}

pub fn network_stop() -> String {
    "(function(){if(window.__hapNetOrigFetch){window.fetch=window.__hapNetOrigFetch;delete window.__hapNetOrigFetch}if(window.__hapNetOrigXHR){window.XMLHttpRequest=window.__hapNetOrigXHR;delete window.__hapNetOrigXHR}window.__hapNetLog=null;return{stopped:true}})()".into()
}

pub fn storage_get(storage_type: &str, key: Option<&str>) -> String {
    if storage_type == "cookie" {
        if let Some(k) = key {
            let k_json = json_str(k);
            format!("(function(){{var pairs=document.cookie.split(';').map(function(c){{var p=c.trim().split('=');return{{key:p[0],value:decodeURIComponent(p.slice(1).join('='))}}}}).filter(function(p){{return p.key}});var k={k_json};var f=pairs.find(function(p){{return p.key===k}});return f?{{key:f.key,value:f.value}}:{{found:false}}}})()")
        } else {
            "(function(){var pairs=document.cookie.split(';').map(function(c){var p=c.trim().split('=');return{key:p[0],value:decodeURIComponent(p.slice(1).join('='))}}).filter(function(p){return p.key});return{type:'cookie',entries:pairs,count:pairs.length}})()".into()
        }
    } else {
        let store = if storage_type == "session" { "sessionStorage" } else { "localStorage" };
        if let Some(k) = key {
            let k_json = json_str(k);
            format!("(function(){{var v={store}.getItem({k_json});return v===null?{{found:false}}:{{key:{k_json},value:v}}}})()")
        } else {
            let type_json = json_str(storage_type);
            format!("(function(){{var s={store};var entries=[];for(var i=0;i<s.length;i++){{var k=s.key(i);entries.push({{key:k,value:s.getItem(k)}})}}return{{type:{type_json},entries:entries,count:entries.length}}}})()")
        }
    }
}

pub fn storage_set(body: &Value) -> String {
    let stype = body.get("type").and_then(|t| t.as_str()).unwrap_or("local");
    let store = if stype == "session" { "sessionStorage" } else { "localStorage" };
    let action = body.get("action").and_then(|a| a.as_str()).unwrap_or("");

    match action {
        "clear" => {
            if stype == "cookie" {
                "(function(){document.cookie.split(';').forEach(function(c){document.cookie=c.trim().split('=')[0]+'=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/'});return{success:true}})()".into()
            } else {
                format!("(function(){{{store}.clear();return{{success:true}}}})()")
            }
        }
        "remove" => {
            let key = body.get("key").and_then(|k| k.as_str()).unwrap_or("");
            if key.is_empty() { return "({error:'key required for remove'})".into(); }
            let k_json = json_str(key);
            if stype == "cookie" {
                format!("(function(){{document.cookie={k_json}+'=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/';return{{success:true}}}})()")
            } else {
                format!("(function(){{{store}.removeItem({k_json});return{{success:true}}}})()")
            }
        }
        "set" => {
            let key = body.get("key").and_then(|k| k.as_str()).unwrap_or("");
            if key.is_empty() { return "({error:'key required for set'})".into(); }
            let k_json = json_str(key);
            let val = body.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let v_json = json_str(val);
            if stype == "cookie" {
                format!("(function(){{document.cookie={k_json}+'='+encodeURIComponent({v_json})+';path=/';return{{success:true}}}})()")
            } else {
                format!("(function(){{{store}.setItem({k_json},{v_json});return{{success:true}}}})()")
            }
        }
        _ => "({error:'unknown action'})".into(),
    }
}

pub fn mock_set(module: &str, command: &str, response: &Value) -> String {
    let key = json_str(&format!("{}::{}", module, command));
    let resp = serde_json::to_string(response).unwrap_or("null".into());
    format!("(function(){{if(!window.__hapMocks){{window.__hapMocks={{}};var orig=window.hap&&window.hap.hal;if(orig){{window.__hapOrigHal=orig;window.hap.hal=function(mod,cmd){{var k=mod+'::'+cmd;if(window.__hapMocks&&window.__hapMocks[k]!==undefined){{return Promise.resolve(JSON.parse(JSON.stringify(window.__hapMocks[k])))}}return orig.apply(this,arguments)}}}}}}window.__hapMocks[{key}]={resp};return{{mocked:true,key:{key}}}}})()")
}

pub fn mock_clear(module: Option<&str>, command: Option<&str>) -> String {
    let m = module.map(json_str).unwrap_or("null".into());
    let c = command.map(json_str).unwrap_or("null".into());
    format!("(function(){{if(!window.__hapMocks)return{{cleared:0}};var m={m};var c={c};if(m&&c){{delete window.__hapMocks[m+'::'+c];return{{cleared:1}}}}if(m){{var n=0;Object.keys(window.__hapMocks).forEach(function(k){{if(k.startsWith(m+'::')){{delete window.__hapMocks[k];n++}}}});return{{cleared:n}}}}var n=Object.keys(window.__hapMocks).length;window.__hapMocks={{}};return{{cleared:n}}}})()")
}

pub fn mock_list() -> String {
    "(function(){var m=window.__hapMocks||{};return{mocks:Object.keys(m).map(function(k){var p=k.split('::');return{module:p[0],command:p[1],response:m[k]}})}})()".into()
}

pub fn batch_step(step: &Value) -> Option<String> {
    let action = step.get("action").and_then(|a| a.as_str())?;
    match action {
        "click" => Some(click(step)),
        "type" => Some(type_text(step)),
        "scroll" => Some(scroll(step)),
        "eval" => step.get("code").and_then(|c| c.as_str()).map(|s| s.to_string()),
        "wait" => {
            let ms = step.get("ms").and_then(|m| m.as_u64()).unwrap_or(1000).min(10000);
            Some(format!("new Promise(function(r){{setTimeout(r,{ms})}}).then(function(){{return{{waited:{ms}}}}})"))
        }
        "query" => step.get("selector").and_then(|s| s.as_str()).map(|sel| {
            dom_query(sel, step.get("type").and_then(|t| t.as_str()).unwrap_or("auto"), false, 1, false)
        }),
        _ => None,
    }
}

use crate::components::layout::Layout;
use crate::store::use_auth;
use leptos::prelude::*;

// ──────────────────────────────────────────────────────────────────────────────
// Data model
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct FilterExample {
    filter: &'static str,
    desc: &'static str,
}

#[derive(Clone)]
struct FilterSection {
    title: &'static str,
    icon: &'static str,
    items: Vec<FilterExample>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Guide page
// ──────────────────────────────────────────────────────────────────────────────

fn sections() -> Vec<FilterSection> {
    vec![
        FilterSection {
            title: "协议过滤",
            icon: "🌐",
            items: vec![
                FilterExample {
                    filter: "tcp",
                    desc: "所有 TCP 包",
                },
                FilterExample {
                    filter: "udp",
                    desc: "所有 UDP 包",
                },
                FilterExample {
                    filter: "icmp",
                    desc: "所有 ICMP 包（ping）",
                },
                FilterExample {
                    filter: "icmpv6",
                    desc: "IPv6 的 ICMP 包",
                },
                FilterExample {
                    filter: "http",
                    desc: "HTTP/1.x 请求和响应",
                },
                FilterExample {
                    filter: "http2",
                    desc: "HTTP/2 流量",
                },
                FilterExample {
                    filter: "dns",
                    desc: "DNS 查询与响应",
                },
                FilterExample {
                    filter: "tls",
                    desc: "TLS/SSL 握手和加密流量",
                },
                FilterExample {
                    filter: "ssh",
                    desc: "SSH 连接流量",
                },
                FilterExample {
                    filter: "ftp",
                    desc: "FTP 控制连接",
                },
                FilterExample {
                    filter: "ftp-data",
                    desc: "FTP 数据传输",
                },
                FilterExample {
                    filter: "smtp",
                    desc: "SMTP 邮件流量",
                },
                FilterExample {
                    filter: "imap",
                    desc: "IMAP 邮件流量",
                },
                FilterExample {
                    filter: "pop",
                    desc: "POP3 邮件流量",
                },
                FilterExample {
                    filter: "arp",
                    desc: "ARP 地址解析包",
                },
                FilterExample {
                    filter: "bootp",
                    desc: "DHCP 包（基于 BOOTP）",
                },
                FilterExample {
                    filter: "ntp",
                    desc: "NTP 时间同步包",
                },
                FilterExample {
                    filter: "sip",
                    desc: "SIP 信令（VoIP）",
                },
                FilterExample {
                    filter: "rtp",
                    desc: "RTP 媒体流（VoIP）",
                },
                FilterExample {
                    filter: "mysql",
                    desc: "MySQL 数据库协议",
                },
                FilterExample {
                    filter: "redis",
                    desc: "Redis 协议",
                },
                FilterExample {
                    filter: "ptp",
                    desc: "PTPv2 精确时间协议（IEEE 1588，用于局域网时钟同步）",
                },
                FilterExample {
                    filter: "lldp",
                    desc: "LLDP 链路层发现协议（交换机/路由器邻居发现）",
                },
                FilterExample {
                    filter: "stp",
                    desc: "STP/RSTP 生成树协议（防止二层环路）",
                },
                FilterExample {
                    filter: "ospf",
                    desc: "OSPF 路由协议",
                },
                FilterExample {
                    filter: "bgp",
                    desc: "BGP 边界网关协议（互联网路由）",
                },
                FilterExample {
                    filter: "dhcpv6",
                    desc: "DHCPv6 IPv6 地址分配",
                },
                FilterExample {
                    filter: "mdns",
                    desc: "mDNS 多播 DNS（局域网服务发现，如 Bonjour）",
                },
                FilterExample {
                    filter: "llmnr",
                    desc: "LLMNR 本地链路多播名称解析（Windows 局域网）",
                },
                FilterExample {
                    filter: "nbns",
                    desc: "NetBIOS 名称服务（Windows 工作组发现）",
                },
            ],
        },
        FilterSection {
            title: "PTPv2 精确时间协议",
            icon: "⏰",
            items: vec![
                FilterExample {
                    filter: "ptp",
                    desc: "所有 PTPv2 包（IEEE 1588-2008，精确到纳秒级时钟同步）",
                },
                FilterExample {
                    filter: "ptp.v2.messageid == 0x0",
                    desc: "Sync 消息——主时钟发出的同步报文",
                },
                FilterExample {
                    filter: "ptp.v2.messageid == 0x1",
                    desc: "Delay_Req 消息——从时钟请求测量延迟",
                },
                FilterExample {
                    filter: "ptp.v2.messageid == 0x8",
                    desc: "Follow_Up 消息——携带精确发送时间戳",
                },
                FilterExample {
                    filter: "ptp.v2.messageid == 0x9",
                    desc: "Delay_Resp 消息——主时钟回应延迟测量",
                },
                FilterExample {
                    filter: "ptp.v2.messageid == 0xb",
                    desc: "Announce 消息——主时钟宣告自身（BMCA 选主）",
                },
                FilterExample {
                    filter: "ptp and eth.dst == 01:1b:19:00:00:00",
                    desc: "PTPv2 多播包（默认多播地址）",
                },
                FilterExample {
                    filter: "ptp.v2.domainnumber == 0",
                    desc: "PTP 域 0（默认域，最常见）",
                },
            ],
        },
        FilterSection {
            title: "IP 地址过滤",
            icon: "📍",
            items: vec![
                FilterExample {
                    filter: "ip.addr == 192.168.1.1",
                    desc: "源或目标为该 IP 的所有包",
                },
                FilterExample {
                    filter: "ip.src == 192.168.1.1",
                    desc: "源 IP 为该地址的包",
                },
                FilterExample {
                    filter: "ip.dst == 192.168.1.1",
                    desc: "目标 IP 为该地址的包",
                },
                FilterExample {
                    filter: "ip.addr == 10.0.0.0/8",
                    desc: "10.x.x.x 网段的所有包",
                },
                FilterExample {
                    filter: "ip.addr == 192.168.0.0/16",
                    desc: "192.168.x.x 内网包",
                },
                FilterExample {
                    filter: "not ip.addr == 192.168.1.1",
                    desc: "不包含该 IP 的包",
                },
                FilterExample {
                    filter: "ip.src == 1.1.1.1 and ip.dst == 8.8.8.8",
                    desc: "指定源到目标的单向流量",
                },
                FilterExample {
                    filter: "ipv6",
                    desc: "所有 IPv6 包",
                },
                FilterExample {
                    filter: "ipv6.addr == ::1",
                    desc: "IPv6 回环地址",
                },
            ],
        },
        FilterSection {
            title: "端口过滤",
            icon: "🔌",
            items: vec![
                FilterExample {
                    filter: "tcp.port == 80",
                    desc: "HTTP 端口（源或目标）",
                },
                FilterExample {
                    filter: "tcp.port == 443",
                    desc: "HTTPS 端口",
                },
                FilterExample {
                    filter: "tcp.port == 22",
                    desc: "SSH 端口",
                },
                FilterExample {
                    filter: "tcp.port == 3306",
                    desc: "MySQL 端口",
                },
                FilterExample {
                    filter: "tcp.port == 5432",
                    desc: "PostgreSQL 端口",
                },
                FilterExample {
                    filter: "tcp.port == 6379",
                    desc: "Redis 端口",
                },
                FilterExample {
                    filter: "tcp.port == 8080",
                    desc: "常用 HTTP 备用端口",
                },
                FilterExample {
                    filter: "tcp.dstport == 443",
                    desc: "目标端口为 443（HTTPS 请求方）",
                },
                FilterExample {
                    filter: "tcp.srcport == 443",
                    desc: "源端口为 443（HTTPS 响应方）",
                },
                FilterExample {
                    filter: "udp.port == 53",
                    desc: "DNS（UDP）端口",
                },
                FilterExample {
                    filter: "udp.port == 67 or udp.port == 68",
                    desc: "DHCP 端口（服务器/客户端）",
                },
                FilterExample {
                    filter: "tcp.port >= 1024 and tcp.port <= 65535",
                    desc: "非特权端口范围",
                },
            ],
        },
        FilterSection {
            title: "TCP 标志位过滤",
            icon: "🚩",
            items: vec![
                FilterExample {
                    filter: "tcp.flags.syn == 1 and tcp.flags.ack == 0",
                    desc: "TCP 连接请求（SYN 包，三次握手第一步）",
                },
                FilterExample {
                    filter: "tcp.flags.syn == 1 and tcp.flags.ack == 1",
                    desc: "TCP 连接确认（SYN-ACK，第二步）",
                },
                FilterExample {
                    filter: "tcp.flags.fin == 1",
                    desc: "TCP 连接正常关闭（FIN 包）",
                },
                FilterExample {
                    filter: "tcp.flags.rst == 1",
                    desc: "TCP 连接被强制重置（RST 包，异常断开）",
                },
                FilterExample {
                    filter: "tcp.flags.push == 1",
                    desc: "PSH 标志——立即推送数据",
                },
                FilterExample {
                    filter: "tcp.flags.urg == 1",
                    desc: "URG 紧急数据",
                },
                FilterExample {
                    filter: "tcp.flags == 0x002",
                    desc: "纯 SYN 包（十六进制标志匹配）",
                },
                FilterExample {
                    filter: "tcp.flags == 0x012",
                    desc: "SYN-ACK 包",
                },
                FilterExample {
                    filter: "tcp.flags == 0x010",
                    desc: "纯 ACK 包",
                },
                FilterExample {
                    filter: "tcp.analysis.retransmission",
                    desc: "TCP 重传包（网络质量差）",
                },
                FilterExample {
                    filter: "tcp.analysis.duplicate_ack",
                    desc: "重复 ACK（可能有丢包）",
                },
                FilterExample {
                    filter: "tcp.analysis.zero_window",
                    desc: "TCP 窗口为零（接收方缓冲区满）",
                },
                FilterExample {
                    filter: "tcp.analysis.fast_retransmission",
                    desc: "快速重传",
                },
            ],
        },
        FilterSection {
            title: "HTTP 过滤",
            icon: "🌍",
            items: vec![
                FilterExample {
                    filter: "http.request",
                    desc: "所有 HTTP 请求",
                },
                FilterExample {
                    filter: "http.response",
                    desc: "所有 HTTP 响应",
                },
                FilterExample {
                    filter: "http.request.method == \"GET\"",
                    desc: "HTTP GET 请求",
                },
                FilterExample {
                    filter: "http.request.method == \"POST\"",
                    desc: "HTTP POST 请求",
                },
                FilterExample {
                    filter: "http.request.method == \"PUT\"",
                    desc: "HTTP PUT 请求",
                },
                FilterExample {
                    filter: "http.response.code == 200",
                    desc: "HTTP 200 成功响应",
                },
                FilterExample {
                    filter: "http.response.code == 404",
                    desc: "HTTP 404 未找到",
                },
                FilterExample {
                    filter: "http.response.code == 500",
                    desc: "HTTP 500 服务器错误",
                },
                FilterExample {
                    filter: "http.response.code >= 400",
                    desc: "所有 HTTP 错误响应（4xx/5xx）",
                },
                FilterExample {
                    filter: "http.host contains \"example.com\"",
                    desc: "请求特定域名",
                },
                FilterExample {
                    filter: "http.request.uri contains \"/api\"",
                    desc: "URL 路径包含 /api",
                },
                FilterExample {
                    filter: "http.content_type contains \"json\"",
                    desc: "JSON 格式响应",
                },
                FilterExample {
                    filter: "http.authorization",
                    desc: "携带 Authorization 头部的请求",
                },
                FilterExample {
                    filter: "http.cookie",
                    desc: "携带 Cookie 的请求",
                },
                FilterExample {
                    filter: "http.set_cookie",
                    desc: "服务器设置 Cookie 的响应",
                },
                FilterExample {
                    filter: "http.user_agent contains \"curl\"",
                    desc: "curl 工具发出的请求",
                },
            ],
        },
        FilterSection {
            title: "DNS 过滤",
            icon: "🔤",
            items: vec![
                FilterExample {
                    filter: "dns",
                    desc: "所有 DNS 流量",
                },
                FilterExample {
                    filter: "dns.qry.name == \"example.com\"",
                    desc: "查询特定域名",
                },
                FilterExample {
                    filter: "dns.qry.name contains \"google\"",
                    desc: "域名包含关键词",
                },
                FilterExample {
                    filter: "dns.flags.response == 0",
                    desc: "DNS 查询（请求方向）",
                },
                FilterExample {
                    filter: "dns.flags.response == 1",
                    desc: "DNS 响应（回答方向）",
                },
                FilterExample {
                    filter: "dns.flags.rcode != 0",
                    desc: "DNS 错误响应（NXDOMAIN/SERVFAIL 等）",
                },
                FilterExample {
                    filter: "dns.flags.rcode == 3",
                    desc: "NXDOMAIN——域名不存在",
                },
                FilterExample {
                    filter: "dns.qry.type == 1",
                    desc: "A 记录查询（IPv4）",
                },
                FilterExample {
                    filter: "dns.qry.type == 28",
                    desc: "AAAA 记录查询（IPv6）",
                },
                FilterExample {
                    filter: "dns.qry.type == 15",
                    desc: "MX 邮件交换记录查询",
                },
                FilterExample {
                    filter: "dns.qry.type == 16",
                    desc: "TXT 记录查询",
                },
                FilterExample {
                    filter: "dns.a == 1.1.1.1",
                    desc: "解析结果为指定 IP",
                },
            ],
        },
        FilterSection {
            title: "TLS/SSL 过滤",
            icon: "🔒",
            items: vec![
                FilterExample {
                    filter: "tls",
                    desc: "所有 TLS 流量",
                },
                FilterExample {
                    filter: "tls.handshake",
                    desc: "TLS 握手包",
                },
                FilterExample {
                    filter: "tls.handshake.type == 1",
                    desc: "TLS Client Hello（客户端发起）",
                },
                FilterExample {
                    filter: "tls.handshake.type == 2",
                    desc: "TLS Server Hello（服务端回应）",
                },
                FilterExample {
                    filter: "tls.handshake.type == 11",
                    desc: "TLS Certificate（证书交换）",
                },
                FilterExample {
                    filter: "tls.alert",
                    desc: "TLS 警告/错误包",
                },
                FilterExample {
                    filter: "tls.record.content_type == 23",
                    desc: "TLS Application Data（加密数据）",
                },
                FilterExample {
                    filter: "ssl.handshake.extensions_server_name contains \"example.com\"",
                    desc: "SNI 服务器名称指示",
                },
            ],
        },
        FilterSection {
            title: "逻辑运算符",
            icon: "🔣",
            items: vec![
                FilterExample {
                    filter: "tcp and http",
                    desc: "AND：同时满足两个条件",
                },
                FilterExample {
                    filter: "http or dns",
                    desc: "OR：满足任意一个条件",
                },
                FilterExample {
                    filter: "not arp",
                    desc: "NOT：排除某类包",
                },
                FilterExample {
                    filter: "!(ip.addr == 192.168.1.1)",
                    desc: "NOT（括号形式）",
                },
                FilterExample {
                    filter: "tcp.port == 80 and http.request",
                    desc: "80 端口的 HTTP 请求",
                },
                FilterExample {
                    filter: "dns or (tcp.port == 53)",
                    desc: "所有 DNS（UDP+TCP）",
                },
                FilterExample {
                    filter: "ip.src == 10.0.0.1 and (tcp or udp)",
                    desc: "某 IP 的 TCP 和 UDP 流量",
                },
                FilterExample {
                    filter: "(http or dns) and not ip.addr == 127.0.0.1",
                    desc: "排除本地回环的 HTTP/DNS",
                },
            ],
        },
        FilterSection {
            title: "比较运算符",
            icon: "⚖️",
            items: vec![
                FilterExample {
                    filter: "frame.len > 1000",
                    desc: "包长度大于 1000 字节",
                },
                FilterExample {
                    filter: "frame.len < 100",
                    desc: "包长度小于 100 字节（可能是控制包）",
                },
                FilterExample {
                    filter: "frame.len == 60",
                    desc: "包长度精确等于 60 字节",
                },
                FilterExample {
                    filter: "tcp.len > 0",
                    desc: "包含 TCP 数据（非纯 ACK）",
                },
                FilterExample {
                    filter: "ip.ttl < 10",
                    desc: "TTL 很小（可能经过多跳或被篡改）",
                },
                FilterExample {
                    filter: "ip.ttl == 64",
                    desc: "TTL = 64（Linux 默认值）",
                },
                FilterExample {
                    filter: "ip.ttl == 128",
                    desc: "TTL = 128（Windows 默认值）",
                },
                FilterExample {
                    filter: "tcp.window_size == 0",
                    desc: "TCP 窗口为零",
                },
                FilterExample {
                    filter: "tcp.seq == 0",
                    desc: "序列号为 0（通常是 SYN）",
                },
            ],
        },
        FilterSection {
            title: "字符串匹配",
            icon: "🔍",
            items: vec![
                FilterExample {
                    filter: "frame contains \"password\"",
                    desc: "整个包内容包含 password（明文搜索）",
                },
                FilterExample {
                    filter: "http contains \"login\"",
                    desc: "HTTP 包含 login 字符串",
                },
                FilterExample {
                    filter: "http.request.uri matches \"^/api/v[0-9]\"",
                    desc: "URI 正则匹配 /api/vN 路径",
                },
                FilterExample {
                    filter: "dns.qry.name matches \".*\\.cn$\"",
                    desc: "DNS 查询以 .cn 结尾的域名",
                },
                FilterExample {
                    filter: "http.host matches \"\\.(cn|com\\.cn)$\"",
                    desc: "访问中国域名",
                },
            ],
        },
        FilterSection {
            title: "时间与帧编号",
            icon: "⏱️",
            items: vec![
                FilterExample {
                    filter: "frame.number <= 100",
                    desc: "前 100 个包",
                },
                FilterExample {
                    filter: "frame.number >= 500 and frame.number <= 600",
                    desc: "第 500-600 个包",
                },
                FilterExample {
                    filter: "frame.time_relative > 5.0",
                    desc: "捕获开始 5 秒后的包",
                },
                FilterExample {
                    filter: "frame.time_delta > 1.0",
                    desc: "与上一包间隔超过 1 秒（可能有延迟）",
                },
                FilterExample {
                    filter: "tcp.time_delta > 0.1",
                    desc: "TCP 流内响应时间超过 100ms",
                },
            ],
        },
        FilterSection {
            title: "实用场景示例",
            icon: "💡",
            items: vec![
                FilterExample {
                    filter: "tcp.flags.syn == 1 and tcp.flags.ack == 0",
                    desc: "🔍 端口扫描检测：大量 SYN 包（无 ACK）是扫描特征",
                },
                FilterExample {
                    filter: "tcp.analysis.retransmission or tcp.analysis.fast_retransmission",
                    desc: "🌐 网络质量分析：找出所有重传包",
                },
                FilterExample {
                    filter: "http.response.code >= 400 or http.response.code >= 500",
                    desc: "🚨 HTTP 错误排查：所有 4xx/5xx 错误",
                },
                FilterExample {
                    filter: "dns.flags.rcode != 0",
                    desc: "🔤 DNS 解析失败：找出所有 DNS 错误响应",
                },
                FilterExample {
                    filter: "tcp.analysis.zero_window or tcp.analysis.window_update",
                    desc: "📉 带宽瓶颈检测：TCP 窗口为零说明接收方缓冲已满",
                },
                FilterExample {
                    filter: "tcp.port == 3306 and mysql",
                    desc: "🗄️ 数据库流量分析：MySQL 查询与响应",
                },
                FilterExample {
                    filter: "not (arp or icmp or dns) and tcp",
                    desc: "🧹 清理噪声：过滤掉广播包，只看 TCP 业务流量",
                },
                FilterExample {
                    filter: "ip.addr == 192.168.1.100 and tcp.flags.rst == 1",
                    desc: "❌ 连接被拒绝：某 IP 发出的 RST 包",
                },
                FilterExample {
                    filter: "http.request.method == \"POST\" and http contains \"password\"",
                    desc: "⚠️ 明文密码检测：HTTP POST 中含有 password 字符串",
                },
                FilterExample {
                    filter: "tls.handshake.type == 1",
                    desc: "🔐 HTTPS 连接建立：监控所有 TLS 新连接",
                },
                FilterExample {
                    filter: "frame.len > 1400 and tcp",
                    desc: "📦 大包检测：接近 MTU 的包（可能触发分片）",
                },
                FilterExample {
                    filter: "ip.src == 10.0.0.5 and not ip.dst == 10.0.0.0/8",
                    desc: "🌏 出向流量：某内网 IP 访问外网的流量",
                },
            ],
        },
    ]
}

#[component]
pub fn CaptureGuidePage() -> impl IntoView {
    let auth = use_auth();
    Effect::new(move |_| {
        if !auth.get().is_logged_in() {
            web_sys::window()
                .unwrap()
                .location()
                .set_href("/login")
                .ok();
        }
    });

    let search = RwSignal::new(String::new());

    let data = sections();

    view! {
        <Layout>
            <div class="max-w-5xl mx-auto">
                // 页眉
                <div class="flex items-center gap-4 mb-6">
                    <a href="/captures" class="text-gray-500 hover:text-gray-800 dark:text-gray-400 dark:hover:text-white transition-colors text-sm">
                        "← 抓包任务"
                    </a>
                    <div>
                        <h1 class="text-2xl font-bold">"📖 Wireshark 过滤器使用指南"</h1>
                        <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">"在分析页面的过滤器输入框中使用以下表达式，按 Enter 或点击「应用过滤」"</p>
                        <p class="text-xs text-gray-400 dark:text-gray-500 mt-1">"💡 数据包列表仅显示 IP 地址，端口信息见「信息」列（如 35976 → 22）或使用 tcp.port == N 过滤"</p>
                    </div>
                </div>

                // 快速参考卡片
                <div class="grid grid-cols-3 gap-3 mb-6">
                    <div class="bg-blue-50 border border-blue-200 dark:bg-blue-900/30 dark:border-blue-700/50 rounded-xl p-4">
                        <div class="text-xs text-blue-600 dark:text-blue-400 font-semibold mb-2 uppercase tracking-wide">"运算符"</div>
                        <div class="space-y-1 font-mono text-xs">
                            <div><span class="text-yellow-600 dark:text-yellow-300">"and"</span><span class="text-gray-400">" / "</span><span class="text-yellow-600 dark:text-yellow-300">"&&"</span><span class="text-gray-500">"  逻辑与"</span></div>
                            <div><span class="text-yellow-600 dark:text-yellow-300">"or"</span><span class="text-gray-400">" / "</span><span class="text-yellow-600 dark:text-yellow-300">"||"</span><span class="text-gray-500">"   逻辑或"</span></div>
                            <div><span class="text-yellow-600 dark:text-yellow-300">"not"</span><span class="text-gray-400">" / "</span><span class="text-yellow-600 dark:text-yellow-300">"!"</span><span class="text-gray-500">"   取反"</span></div>
                        </div>
                    </div>
                    <div class="bg-green-50 border border-green-200 dark:bg-green-900/30 dark:border-green-700/50 rounded-xl p-4">
                        <div class="text-xs text-green-600 dark:text-green-400 font-semibold mb-2 uppercase tracking-wide">"比较符"</div>
                        <div class="space-y-1 font-mono text-xs">
                            <div><span class="text-yellow-600 dark:text-yellow-300">"=="</span><span class="text-gray-500">"  等于"</span></div>
                            <div><span class="text-yellow-600 dark:text-yellow-300">"!="</span><span class="text-gray-500">"  不等于"</span></div>
                            <div><span class="text-yellow-600 dark:text-yellow-300">">"</span><span class="text-gray-400">" / "</span><span class="text-yellow-600 dark:text-yellow-300">"<"</span><span class="text-gray-400">" / "</span><span class="text-yellow-600 dark:text-yellow-300">">="</span><span class="text-gray-400">" / "</span><span class="text-yellow-600 dark:text-yellow-300">"<="</span></div>
                        </div>
                    </div>
                    <div class="bg-purple-50 border border-purple-200 dark:bg-purple-900/30 dark:border-purple-700/50 rounded-xl p-4">
                        <div class="text-xs text-purple-600 dark:text-purple-400 font-semibold mb-2 uppercase tracking-wide">"字符串"</div>
                        <div class="space-y-1 font-mono text-xs">
                            <div><span class="text-yellow-600 dark:text-yellow-300">"contains"</span><span class="text-gray-500">" 包含"</span></div>
                            <div><span class="text-yellow-600 dark:text-yellow-300">"matches"</span><span class="text-gray-500">"  正则匹配"</span></div>
                            <div><span class="text-gray-500">"字符串用双引号括起"</span></div>
                        </div>
                    </div>
                </div>

                // 搜索框
                <div class="mb-6">
                    <input
                        class="w-full input rounded-lg px-4 py-2.5 text-sm font-mono text-gray-900 dark:text-white focus:outline-none"
                        placeholder="搜索过滤器... 例如: tcp, http, port"
                        on:input=move |ev| search.set(event_target_value(&ev))
                    />
                </div>

                // 过滤器分类列表
                <div class="space-y-6">
                    {
                        let data_clone = data.clone();
                        move || {
                            let q = search.get().to_lowercase();
                            data_clone.iter().filter_map(|section| {
                                let filtered: Vec<&FilterExample> = section.items.iter().filter(|ex| {
                                    q.is_empty()
                                        || ex.filter.to_lowercase().contains(&q)
                                        || ex.desc.to_lowercase().contains(&q)
                                }).collect();

                                if filtered.is_empty() { return None; }

                                Some(view! {
                                    <div class="card rounded-xl overflow-hidden">
                                        // 章节标题
                                        <div class="px-5 py-3 table-head flex items-center gap-2">
                                            <span class="text-lg">{section.icon}</span>
                                            <span class="font-semibold text-gray-800 dark:text-white">{section.title}</span>
                                            <span class="ml-auto text-xs text-gray-500 bg-gray-100 dark:bg-gray-700 rounded-full px-2 py-0.5">
                                                {filtered.len()}" 条"
                                            </span>
                                        </div>
                                        // 条目列表
                                        <div class="divide-y divide-gray-100 dark:divide-gray-800">
                                            {
                                                filtered.into_iter().map(|ex| {
                                                    let filter_str = ex.filter.to_string();
                                                    let filter_for_copy = filter_str.clone();
                                                    view! {
                                                        <div class="flex items-start gap-4 px-5 py-3 hover:bg-gray-50 dark:hover:bg-gray-800/50 group transition-colors">
                                                            // 过滤器代码
                                                            <div class="flex-1 min-w-0">
                                                                <code class="text-sm font-mono text-green-700 dark:text-green-400 break-all leading-relaxed">
                                                                    {filter_str}
                                                                </code>
                                                                <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">{ex.desc}</p>
                                                            </div>
                                                            // 复制按钮
                                                            <button
                                                                class="shrink-0 opacity-0 group-hover:opacity-100 transition-opacity text-xs btn-secondary rounded px-2 py-1"
                                                                on:click=move |_| {
                                                                    let val = filter_for_copy.clone();
                                                                    if let Some(window) = web_sys::window() {
                                                                        let _ = window.navigator().clipboard().write_text(&val);
                                                                    }
                                                                }
                                                            >
                                                                "复制"
                                                            </button>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()
                                            }
                                        </div>
                                    </div>
                                })
                            }).collect::<Vec<_>>()
                        }
                    }
                </div>

                // 底部提示
                <div class="mt-8 p-4 bg-yellow-50 border border-yellow-200 dark:bg-yellow-900/20 dark:border-yellow-700/40 rounded-xl text-sm text-yellow-800 dark:text-yellow-300/80">
                    <p class="font-semibold mb-1">"💡 提示"</p>
                    <ul class="space-y-1 text-xs list-disc list-inside text-yellow-700 dark:text-yellow-200/60">
                        <li>"过滤器区分大小写，协议名全部小写（如 tcp 而非 TCP）"</li>
                        <li>"使用括号 () 改变运算优先级，and 优先级高于 or"</li>
                        <li>"字符串值需要用双引号括起来，如 http.host == \"example.com\""</li>
                        <li>"正则表达式使用 PCRE 语法，matches 关键字"</li>
                        <li>"点击「复制」后可直接粘贴到分析页面的过滤器输入框"</li>
                    </ul>
                </div>

                <div class="mt-4 pb-8"></div>
            </div>
        </Layout>
    }
}

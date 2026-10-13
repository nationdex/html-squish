use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use html_squish::{squish_in_place, SquishConfig};

const SAMPLE_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MindVote - Realtime Polling & Predictions</title>
    <!-- SEO & Performance Meta Tags -->
    <meta name="description" content="Decentralized prediction market and community polling platform.">
    <script type="application/ld+json">
    {
        "@context": "https://schema.org",
        "@type": "WebSite",
        "name": "MindVote",
        "url": "https://mindvote.com",
        "description": "High-performance polling and decision intelligence",
        "publisher": {
            "@type": "Organization",
            "name": "MindVote Foundation",
            "logo": {
                "@type": "ImageObject",
                "url": "https://mindvote.com/logo.png"
            }
        }
    }
    </script>
    <style>
        body {
            margin: 0;
            padding: 0;
            font-family: system-ui, -apple-system, sans-serif;
            background-color: #0b0f19;
            color: #f8fafc;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem 1rem;
        }
        .btn-vote {
            background-color: #3b82f6;
            color: #ffffff;
            padding: 0.75rem 1.5rem;
            border-radius: 0.5rem;
        }
    </style>
</head>
<body>
    <!-- Main Header -->
    <header class = "header sticky"   id = "main-header" >
        <nav class="nav container">
            <a href="/" class="brand-logo">
                <span class="highlight">Mind</span>Vote
            </a>
            <div class="nav-links">
                <a href="/polls">Active Polls</a>
                <a href="/predictions">Predictions</a>
                <a href="/leaderboard">Leaderboard</a>
            </div>
        </nav>
    </header>

    <main class="container">
        <section class="hero">
            <h1>Decentralized Prediction Market</h1>
            <p>
                Cast your vote, earn reputation, and shape collective intelligence.
                Fast, verifiable, zero-latency feedback loop.
            </p>
            <div class="actions">
                <button class="btn-vote" data-id="poll-8921">Vote Now</button>
            </div>
        </section>

        <section class="recent-proposals">
            <h2>Trending Polls</h2>
            <div class="card-grid">
                <div class="poll-card" id="card-1">
                    <h3>Will ETH break $10,000 in 2026?</h3>
                    <p class="votes">Current volume: 1,420,890 VOTE</p>
                </div>
                <div class="poll-card" id="card-2">
                    <h3>ASI Milestone Achievement by Q4</h3>
                    <p class="votes">Current volume: 928,110 VOTE</p>
                </div>
            </div>
        </section>
    </main>

    <script>
        // Realtime WebSocket polling client
        const socket = new WebSocket("wss://api.mindvote.com/live");
        socket.onmessage = (event) => {
            const data = JSON.parse(event.data);
            console.log("Update received:", data);
        };
    </script>
</body>
</html>
"#;

fn bench_squish(c: &mut Criterion) {
    let mut group = c.benchmark_group("html-squish");
    let input_bytes = SAMPLE_HTML.as_bytes();
    group.throughput(Throughput::Bytes(input_bytes.len() as u64));

    let mut buf = vec![0u8; input_bytes.len()];

    group.bench_function("squish_in_place", |b| {
        b.iter(|| {
            buf.copy_from_slice(black_box(input_bytes));
            let len = squish_in_place(&mut buf);
            black_box(len);
        });
    });

    group.bench_function("squish_aggressive", |b| {
        let config = SquishConfig::aggressive();
        b.iter(|| {
            buf.copy_from_slice(black_box(input_bytes));
            let len = html_squish::squish_in_place_with_config(&mut buf, &config);
            black_box(len);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_squish);
criterion_main!(benches);

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use proxy_protocol::version2::parse_proxy_protocol;
use std::time::Duration;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    runtime::Runtime,
    time::sleep,
};

/// 创建 IPv4 PROXY v2 帧
fn create_ipv4_frame() -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend_from_slice(b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A");
    frame.push(0x21);
    frame.push(0x11);
    frame.extend_from_slice(&12u16.to_be_bytes());
    frame.extend_from_slice(&[192, 168, 1, 100]);
    frame.extend_from_slice(&8080u16.to_be_bytes());
    frame.extend_from_slice(&[10, 0, 0, 1]);
    frame.extend_from_slice(&80u16.to_be_bytes());
    frame
}

/// 创建 IPv6 PROXY v2 帧
fn create_ipv6_frame() -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend_from_slice(b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A");
    frame.push(0x21);
    frame.push(0x21);
    frame.extend_from_slice(&36u16.to_be_bytes());
    frame.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    frame.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
    frame.extend_from_slice(&8080u16.to_be_bytes());
    frame.extend_from_slice(&80u16.to_be_bytes());
    frame
}

/// 创建 LOCAL 命令帧
fn create_local_frame() -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend_from_slice(b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A");
    frame.push(0x20);
    frame.push(0x00);
    frame.extend_from_slice(&0u16.to_be_bytes());
    frame
}

/// 创建非 PROXY 协议帧
fn create_non_proxy_frame() -> Vec<u8> {
    b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n".to_vec()
}

/// 辅助函数：创建测试用的 TCP 流（带重试和延迟）
async fn create_test_stream_with_data(data: Vec<u8>) -> TcpStream {
    // 每次创建后添加小延迟，避免端口耗尽
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        sleep(Duration::from_micros(10)).await;
        let mut stream = TcpStream::connect(&addr).await.unwrap();
        stream.write_all(&data).await.unwrap();
        stream.flush().await.unwrap();
        sleep(Duration::from_millis(100)).await;
    });

    let (test_stream, _) = listener.accept().await.unwrap();
    test_stream
}

fn bench_parse_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("proxy_protocol_parse");

    // 降低样本数以减少连接创建频率
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));

    let runtime = Runtime::new().unwrap();

    let scenarios = vec![
        ("ipv4_28bytes", create_ipv4_frame()),
        ("ipv6_52bytes", create_ipv6_frame()),
        ("local_16bytes", create_local_frame()),
        ("non_proxy_detection", create_non_proxy_frame()),
    ];

    for (name, data) in scenarios {
        group.throughput(Throughput::Bytes(data.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(name), &data, |b, data| {
            b.to_async(&runtime).iter(|| async {
                // 在每次迭代之间添加小延迟，避免端口耗尽
                sleep(Duration::from_micros(100)).await;

                let mut stream = create_test_stream_with_data(data.clone()).await;
                let result = parse_proxy_protocol(&mut stream).await.unwrap();
                black_box(result);

                // 确保连接完全关闭
                drop(stream);
                sleep(Duration::from_micros(50)).await;
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_parse_scenarios);
criterion_main!(benches);

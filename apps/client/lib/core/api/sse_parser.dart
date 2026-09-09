import 'dart:async';
import 'dart:convert';

class SseEnvelope {
  const SseEnvelope({
    required this.version,
    required this.event,
    required this.data,
  });

  factory SseEnvelope.fromJson(Map<String, dynamic> json) => SseEnvelope(
        version: json['version'] as int,
        event: json['event'] as String,
        data: json['data'] as Map<String, dynamic>,
      );

  final int version;
  final String event;
  final Map<String, dynamic> data;
}

/// Parses SSE raw lines into JSON envelopes.
/// SSE format from axum: `event: text_delta\ndata: {"version":1,"event":"text_delta","data":{"text":"..."}}`
Stream<SseEnvelope> parseSseStream(Stream<String> lines) async* {
  String? currentData;

  await for (final line in lines) {
    final trimmed = line.trim();
    if (trimmed.isEmpty) {
      if (currentData != null) {
        try {
          final outer = jsonDecode(currentData) as Map<String, dynamic>;
          // axum Sse wraps data as string; our envelope is inside data field
          // The data we sent is JSON string of SseEnvelope, so outer is the envelope
          if (outer.containsKey('event') && outer.containsKey('version')) {
            yield SseEnvelope.fromJson(outer);
          } else if (outer.containsKey('data')) {
            // fallback if double-wrapped
            final inner = outer['data'];
            if (inner is Map<String, dynamic> && inner.containsKey('event')) {
              yield SseEnvelope.fromJson(inner);
            }
          }
        } catch (_) {
          // ignore malformed
        }
        currentData = null;
      }
      continue;
    }
    if (trimmed.startsWith('data:')) {
      currentData = trimmed.substring(5).trim();
    }
  }
}

/// Helper to split Dio ResponseBody stream (Uint8List) into lines
Stream<String> byteStreamToLines(Stream<List<int>> bytes) {
  return bytes
      .transform(utf8.decoder)
      .transform(const LineSplitter());
}

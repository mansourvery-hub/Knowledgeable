import 'dart:async';
import 'dart:convert';
import 'dart:developer' as dev;

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

Stream<SseEnvelope> parseSseStream(Stream<String> lines) async* {
  String? currentData;

  await for (final line in lines) {
    dev.log('SSE Parser: line: $line');
    final trimmed = line.trim();
    if (trimmed.startsWith('data:')) {
      final content = trimmed.substring(5).trim();
      currentData = (currentData == null) ? content : (currentData! + content);
    } else if (trimmed.isEmpty && currentData != null) {
      try {
        // Handle potential partial fragments or double-escaped JSON if the server emits them
        final outer = jsonDecode(currentData!) as Map<String, dynamic>;
        
        // Robustness check: Axum SSE sometimes sends raw data strings, sometimes objects.
        // If 'event' is missing in outer, look inside 'data' if it exists.
        if (outer.containsKey('event') && outer.containsKey('version')) {
          yield SseEnvelope.fromJson(outer);
        } else if (outer.containsKey('data')) {
           final inner = outer['data'];
           if (inner is Map<String, dynamic> && inner.containsKey('event')) {
             yield SseEnvelope.fromJson(inner);
           }
        }
      } catch (e) {
        dev.log('SSE Parser: FAILED TO DECODE: $currentData. Error: $e');
        // Try a more flexible approach
        try {
          final raw = jsonDecode(currentData!) as Map<String, dynamic>;
          // If it doesn't have the envelope fields, try to construct one
          final event = raw['event'] as String? ?? 'unknown';
          final data = raw['data'] is Map<String, dynamic> ? raw['data'] as Map<String, dynamic> : raw;
          
          yield SseEnvelope(
            version: raw['version'] as int? ?? 1,
            event: event,
            data: data,
          );
        } catch (e2) {
          dev.log('SSE Parser: Even flexible parsing failed: $e2');
        }
      }
      currentData = null;
    }
  }
}

Stream<String> byteStreamToLines(Stream<List<int>> bytes) {
  return bytes
      .transform(utf8.decoder)
      .transform(const LineSplitter());
}

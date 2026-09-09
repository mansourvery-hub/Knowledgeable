import 'package:flutter_test/flutter_test.dart';
import 'package:knowledgeable_client/core/api/sse_parser.dart';

void main() {
  group('SseParser Tests', () {
    test('Parses clean text_delta events', () async {
      final lines = Stream.fromIterable([
        'event: text_delta',
        'data: {"version": 1, "event": "text_delta", "data": {"text": "hello"}}',
        '',
        'event: text_delta',
        'data: {"version": 1, "event": "text_delta", "data": {"text": " world"}}',
        '',
      ]);

      final envelopes = await parseSseStream(lines).toList();
      expect(envelopes.length, 2);
      expect(envelopes[0].event, 'text_delta');
      expect(envelopes[0].data['text'], 'hello');
      expect(envelopes[1].event, 'text_delta');
      expect(envelopes[1].data['text'], ' world');
    });

    test('Parses events with spaceless data prefix', () async {
      final lines = Stream.fromIterable([
        'event:text_delta',
        'data:{"version":1,"event":"text_delta","data":{"text":"hello"}}',
        '',
      ]);

      final envelopes = await parseSseStream(lines).toList();
      expect(envelopes.length, 1);
      expect(envelopes[0].event, 'text_delta');
      expect(envelopes[0].data['text'], 'hello');
    });

    test('Ignores malformed json data gracefully', () async {
      final lines = Stream.fromIterable([
        'event: text_delta',
        'data: {malformed_json_here}',
        '',
      ]);

      final envelopes = await parseSseStream(lines).toList();
      expect(envelopes.isEmpty, true);
    });
  });
}

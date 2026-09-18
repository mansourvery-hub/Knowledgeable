import 'package:flutter_test/flutter_test.dart';
import 'package:knowledgeable_client/core/api/sse_parser.dart';

void main() {
  group('SSE Infrastructure', () {
    group('SseParser', () {
      test('Parses clean text_delta events', () async {
        final stream = Stream.value('data: {"version": 1, "event": "text_delta", "data": {"text": "Hello"}}\n\n'.codeUnits);
        final lines = byteStreamToLines(stream);
        final results = await parseSseStream(lines).toList();
        expect(results.length, 1);
        expect(results[0].event, 'text_delta');
      });

      test('Fails gracefully on malformed data', () async {
        final stream = Stream.value('data: {"malformed": "data"}\n\n'.codeUnits);
        final lines = byteStreamToLines(stream);
        final results = await parseSseStream(lines).toList();
        expect(results.length, 0);
      });
    });
  });
}

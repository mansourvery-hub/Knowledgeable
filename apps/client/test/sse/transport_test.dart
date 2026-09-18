import 'package:flutter_test/flutter_test.dart';
import 'package:knowledgeable_client/core/api/sse_parser.dart';

void main() {
  test('SSE Parser handles multi-line data', () async {
    final lines = Stream.fromIterable([
      'data: {"version": 1, "event": "text_delta", "data": {"text": "Hello"}}',
      '',
    ]);

    final envelopes = await parseSseStream(lines).toList();
    print('Envelopes count: ${envelopes.length}');
    if (envelopes.isNotEmpty) print('Data: ${envelopes[0].data}');
    expect(envelopes.length, 1);
    expect(envelopes[0].data['text'], 'Hello');
  });

}

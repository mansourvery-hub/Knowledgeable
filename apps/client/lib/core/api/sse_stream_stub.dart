import 'dart:async';

Stream<List<int>> sseStream(
  String url,
  String body,
  Map<String, String> headers,
) {
  throw UnsupportedError('Cannot stream SSE without html or io platform implementation.');
}

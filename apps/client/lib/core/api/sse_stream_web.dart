import 'dart:async';
import 'dart:html' as html;

Stream<String> sseStream(
  String url,
  String body,
  Map<String, String> headers,
) {
  final controller = StreamController<String>();
  final xhr = html.HttpRequest();
  
  xhr.open('POST', url, async: true);
  headers.forEach((k, v) => xhr.setRequestHeader(k, v));

  int lastLength = 0;
  xhr.onReadyStateChange.listen((_) {
    if (xhr.readyState == 3 || xhr.readyState == 4) {
      final responseText = xhr.responseText ?? '';
      if (responseText.length > lastLength) {
        final chunk = responseText.substring(lastLength);
        lastLength = responseText.length;
        controller.add(chunk);
      }
    }
    if (xhr.readyState == 4) {
      if (!controller.isClosed) {
        controller.close();
      }
    }
  });

  xhr.onError.listen((e) {
    if (!controller.isClosed) {
      controller.addError(Exception('XHR network error or connection closed.'));
      controller.close();
    }
  });

  xhr.send(body);
  return controller.stream;
}

// ignore_for_file: uri_does_not_exist, avoid_web_libraries_in_flutter, deprecated_member_use
import 'dart:async';
import 'dart:html' as html;
import 'dart:js_util' as js_util;
import 'dart:typed_data';
import 'dart:convert';

Stream<String> sseStream(
  String url,
  String body,
  Map<String, String> headers,
) {
  final controller = StreamController<String>();

  final fetchOptions = js_util.newObject();
  js_util.setProperty(fetchOptions, 'method', 'POST');
  js_util.setProperty(fetchOptions, 'body', body);
  
  final headersObj = js_util.newObject();
  headers.forEach((k, v) => js_util.setProperty(headersObj, k, v));
  js_util.setProperty(fetchOptions, 'headers', headersObj);

  js_util.promiseToFuture(html.window.fetch(url, fetchOptions)).then((response) {
    final status = js_util.getProperty(response, 'status') as int? ?? 200;
    if (status < 200 || status >= 300) {
      controller.addError(Exception('Fetch failed with status $status'));
      controller.close();
      return;
    }

    final bodyStream = js_util.getProperty(response, 'body');
    if (bodyStream == null) {
      controller.addError(Exception('Fetch response body is null'));
      controller.close();
      return;
    }
    final reader = js_util.callMethod(bodyStream, 'getReader', []);

    void readChunk() {
      js_util.promiseToFuture(js_util.callMethod(reader, 'read', [])).then((result) {
        final done = js_util.getProperty(result, 'done') as bool? ?? false;
        if (done) {
          if (!controller.isClosed) {
            controller.close();
          }
          return;
        }
        final value = js_util.getProperty(result, 'value');
        if (value != null) {
          final bytes = value as Uint8List;
          final text = utf8.decode(bytes);
          controller.add(text);
        }
        readChunk();
      }).catchError((e) {
        if (!controller.isClosed) {
          controller.addError(e);
          controller.close();
        }
      });
    }

    readChunk();
  }).catchError((e) {
    if (!controller.isClosed) {
      controller.addError(e);
      controller.close();
    }
  });

  return controller.stream;
}

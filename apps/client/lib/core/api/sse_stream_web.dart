// ignore_for_file: uri_does_not_exist, avoid_web_libraries_in_flutter, deprecated_member_use
import 'dart:async';
import 'dart:html' as html;
import 'dart:js_util' as js_util;
import 'dart:convert';

List<int> jsUint8ArrayToDartList(dynamic jsArray) {
  final length = js_util.getProperty(jsArray, 'length') as int? ?? 0;
  final list = List<int>.filled(length, 0);
  for (int i = 0; i < length; i++) {
    list[i] = js_util.getProperty(jsArray, i) as int? ?? 0;
  }
  return list;
}

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

  // Call JS window.fetch directly to get raw JS Promise
  final fetchPromise = js_util.callMethod(html.window, 'fetch', [url, fetchOptions]);

  js_util.promiseToFuture(fetchPromise).then((response) {
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
      final readPromise = js_util.callMethod(reader, 'read', []);
      js_util.promiseToFuture(readPromise).then((result) {
        final done = js_util.getProperty(result, 'done') as bool? ?? false;
        if (done) {
          if (!controller.isClosed) {
            controller.close();
          }
          return;
        }
        final value = js_util.getProperty(result, 'value');
        if (value != null) {
          final bytes = jsUint8ArrayToDartList(value);
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

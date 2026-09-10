// ignore_for_file: uri_does_not_exist, avoid_web_libraries_in_flutter, deprecated_member_use
import 'dart:async';
import 'dart:developer' as dev;
import 'dart:html' as html;
import 'dart:js_util' as js_util;

List<int> jsUint8ArrayToDartList(dynamic jsArray) {
  final length = js_util.getProperty(jsArray, 'length') as int? ?? 0;
  final list = List<int>.filled(length, 0);
  for (int i = 0; i < length; i++) {
    list[i] = js_util.getProperty(jsArray, i) as int? ?? 0;
  }
  return list;
}

Stream<List<int>> sseStream(
  String url,
  String body,
  Map<String, String> headers,
) {
  final controller = StreamController<List<int>>();

  final fetchOptions = js_util.newObject();
  js_util.setProperty(fetchOptions, 'method', 'POST');
  js_util.setProperty(fetchOptions, 'body', body);
  
  final headersObj = js_util.newObject();
  headers.forEach((k, v) => js_util.setProperty(headersObj, k, v));
  js_util.setProperty(fetchOptions, 'headers', headersObj);

  // Call JS window.fetch directly to get raw JS Promise
  final fetchPromise = js_util.callMethod(html.window, 'fetch', [url, fetchOptions]);

  js_util.promiseToFuture(fetchPromise).then((response) {
    dev.log('SSE fetch response: $response');
    dev.log('SSE status: ${js_util.getProperty(response, 'status')}');
    final status = js_util.getProperty(response, 'status') as int? ?? 200;
    if (status < 200 || status >= 300) {
      controller.addError(Exception('Fetch failed with status $status'));
      controller.close();
      return;
    }

    final bodyStream = js_util.getProperty(response, 'body');
    dev.log('SSE response body: $bodyStream');
    if (bodyStream == null) {
      controller.addError(Exception('Fetch response body is null'));
      controller.close();
      return;
    }
    final reader = js_util.callMethod(bodyStream, 'getReader', []);
    var terminated = false;

    Future<void> terminate() async {
      if (terminated) return;
      terminated = true;
      try {
        await js_util.promiseToFuture(js_util.callMethod(reader, 'cancel', []));
      } catch (_) {}
      if (!controller.isClosed) {
        await controller.close();
      }
    }

    Future<void> pump() async {
      dev.log('SSE fetch started: $url');
      try {
        while (!terminated) {
          final readPromise = js_util.callMethod(reader, 'read', []);
          final result = await js_util.promiseToFuture(readPromise).timeout(
                const Duration(seconds: 30),
                onTimeout: () {
                  dev.log('SSE reader timeout');
                  throw Exception('Stream reader timed out');
                },
              );

          final done = js_util.getProperty(result, 'done');
          final isDone = done == true;

          if (isDone) {
            dev.log('SSE stream completed');
            await terminate();
            return;
          }

          final value = js_util.getProperty(result, 'value');
          if (value == null) {
            dev.log('SSE invalid: value is null');
            controller.addError(Exception('Invalid SSE stream: "value" is null'));
            await terminate();
            return;
          }

          final bytes = jsUint8ArrayToDartList(value);
          if (bytes.isNotEmpty) {
            dev.log('SSE sending bytes: ${bytes.length}');
            controller.add(bytes);
          }
        }
      } catch (e) {
        dev.log('SSE error: $e');
        if (!controller.isClosed) {
          controller.addError(e);
        }
        await terminate();
      }
    }

    controller.onCancel = terminate;
    dev.log('Starting pump...');
    pump();
  }).catchError((e) {
    if (!controller.isClosed) {
      controller.addError(e);
      controller.close();
    }
  });

  return controller.stream;
}

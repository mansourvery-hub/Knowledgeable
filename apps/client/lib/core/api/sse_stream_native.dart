import 'dart:async';
import 'dart:convert';
import 'package:dio/dio.dart';

Stream<String> sseStream(
  String url,
  String body,
  Map<String, String> headers,
) async* {
  final dio = Dio(BaseOptions(
    connectTimeout: const Duration(seconds: 10),
    receiveTimeout: const Duration(seconds: 30),
  ));
  
  final response = await dio.post<ResponseBody>(
    url,
    data: body,
    options: Options(
      headers: headers,
      responseType: ResponseType.stream,
    ),
  );

  final stream = response.data!.stream;
  await for (final chunk in stream) {
    yield utf8.decode(chunk);
  }
}

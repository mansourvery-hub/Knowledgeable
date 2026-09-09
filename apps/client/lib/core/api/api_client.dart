import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Typed API client foundation — all HTTP goes through repositories, never widgets.
/// Per architecture: Flutter must not call LLM providers or mutate graph directly.
class ApiClient {
  ApiClient({String? baseUrl, Dio? dio})
      : _dio = dio ??
            Dio(
              BaseOptions(
                baseUrl: baseUrl ?? const String.fromEnvironment('API_BASE_URL', defaultValue: 'http://localhost:3000'),
                connectTimeout: const Duration(seconds: 10),
                receiveTimeout: const Duration(seconds: 30),
                headers: {'Content-Type': 'application/json'},
              ),
            ) {
    _dio.interceptors.add(LogInterceptor(requestBody: false, responseBody: false));
  }

  final Dio _dio;

  Dio get dio => _dio;

  Future<Response<T>> get<T>(String path, {Map<String, dynamic>? query}) =>
      _dio.get<T>(path, queryParameters: query);

  Future<Response<T>> post<T>(String path, {Object? data}) => _dio.post<T>(path, data: data);
}

final apiClientProvider = Provider<ApiClient>((ref) => ApiClient());

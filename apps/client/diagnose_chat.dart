import 'dart:convert';
import 'package:http/http.dart' as http;

Future<void> main() async {
  final url = Uri.parse('http://127.0.0.1:3000/v1/conversations/00a2111e-932e-42cc-9b13-6d13e3ee234f/messages');
  print('Connecting to: $url');
  
  final request = http.Request('POST', url)
    ..headers['Content-Type'] = 'application/json'
    ..headers['Accept'] = 'text/event-stream'
    ..body = jsonEncode({'content': 'What is a concept on my frontier?'});

  final client = http.Client();
  final response = await client.send(request);

  print('Status: ${response.statusCode}');
  
  await response.stream.transform(utf8.decoder).listen(
    (data) {
      print('RAW CHUNK: $data');
    },
    onError: (e) => print('ERROR: $e'),
    onDone: () => print('STREAM CLOSED'),
  );
}

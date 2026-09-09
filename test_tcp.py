import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
try:
    s.bind(('127.0.0.1', 3000))
    print("Bind successful")
    s.listen(1)
    print("Listen successful")
except Exception as e:
    print(f"Error: {e}")
finally:
    s.close()

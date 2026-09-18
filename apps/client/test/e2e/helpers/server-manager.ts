import { spawn, ChildProcess } from 'child_process';
import path from 'path';

export class ServerManager {
  private backendProcess: ChildProcess | null = null;

  async startBackend() {
    console.log('Starting backend server...');
    const projectRoot = path.resolve(__dirname, '../../../');
    
    // Set environment variables for the test
    const env = { 
        ...process.env, 
        PORT: '3000', 
        DATABASE_URL: 'sqlite::memory:' // Fresh database for every test run
    };

    this.backendProcess = spawn('cargo', ['run', '--bin', 'server'], {
      cwd: projectRoot,
      env,
      stdio: 'pipe',
    });

    this.backendProcess.stdout?.on('data', (data) => console.log(`[Backend]: ${data}`));
    this.backendProcess.stderr?.on('data', (data) => console.error(`[Backend]: ${data}`));

    // Wait for the server to be ready (simplistic approach)
    await new Promise((resolve) => setTimeout(resolve, 5000));
  }

  async stopBackend() {
    if (this.backendProcess) {
      this.backendProcess.kill();
    }
  }
}

export {};

declare global {
  interface Window {
    pywebview?: {
      api: {
        get_installed_browsers(): Promise<BrowserInfo[]>;
        get_connected_browser(): Promise<BrowserInfo | null>;
        get_current_cdp_url(): Promise<string>;
        close_browser(url: string): Promise<boolean>;
        launch_browser(name: string): Promise<void>;
        get_installed_presences(): Promise<any[]>;
        start_presence(name: string): Promise<void>;
        stop_presence(name: string): Promise<void>;
        get_remote_presences(): Promise<RemotePresence[]>;
        install_remote_presence(name: string): Promise<ResultState>;
        remove_installed_presence(name: string): Promise<ResultState>;
        is_discord_running(): Promise<boolean>;
        get_network_processes(): Promise<any[]>;
        close_network_processes(port: number): Promise<boolean>;
        get_user_setting_key(key: string): Promise<any>;
        set_user_setting_key(key: string, value: any): Promise<void>;
        ping?(msg: string): Promise<string>;
      };
    };
  }
}

interface BrowserInfo {
  name: string;
  path: string;
}

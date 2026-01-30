export type Presence = {
  name: string;
  client_id: string;
  web: boolean;
};

export type InstalledPresence = Presence & {
  path: string;
  entrypoint: string;
  callable_name: string;
  interval: number;
  enabled: boolean;
  backoff_time: number;
  on_exit: any;
  runs: number;
  running: boolean;
};

export type RemotePresence = {
  name: string;
  manifest: {
    name: string;
    version: string;
    package: string;
    installed: boolean;
    description: {
      en: string;
    };
    client_id: string;
    image: string;
    entry: string;
    contributors: string[];
    author: string;
    category: string;
    color: string;
    tags: string[];
    web: boolean;
  };
};

import { Event, listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

interface Payload {
  message: string;
  username: string;
}

const ReadChat = () => {
  const [payload, setPayload] = useState<Payload | undefined>(undefined);

  useEffect(() => {
    const unlisten = listen('read-chat', (event: Event<Payload>) => {
      setPayload(event.payload);
    });

    return () => {
      unlisten.then((close) => close());
    };
  }, []);

  if (!payload) return null;

  return (
    <div className="w-screen h-screen text-white flex justify-center flex-col items-center gap-8 px-8">
      <div className="flex gap-2 items-center">
        <img src="https://cdn.7tv.app/emote/60f7372bf7fdd1860a7dcbb2/4x.webp" className="w-16 h-16" />
        <h1 className="text-4xl">PLEASE READ CHAT</h1>
        <img src="https://cdn.7tv.app/emote/60f7372bf7fdd1860a7dcbb2/4x.webp" className="w-16 h-16" />
      </div>
      <div className="text-3xl">
        <i>{payload.username}</i> says:
      </div>
      <div className="text-xl flex-wrap overflow-y-auto text-center">{payload.message}</div>
    </div>
  );
};

export default ReadChat;

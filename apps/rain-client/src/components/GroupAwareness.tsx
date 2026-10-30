import React, { useEffect, useState } from 'react';
import { apiRequest } from '../api';
import { GroupMemory } from '../types';

export function GroupAwareness({ agentId }: { agentId: string }) {
  const [groups, setGroups] = useState<GroupMemory[]>([]);
  const [error, setError] = useState('');
  useEffect(() => {
    let cancelled = false;
    let request = 0;
    setGroups([]); setError('');
    const load = async () => {
      const current = ++request;
      try {
        const response = await apiRequest<GroupMemory[]>(`/api/v1/sessions/${agentId}/groups`);
        if (!cancelled && current === request) { setGroups(response.data); setError(''); }
      } catch { if (!cancelled && current === request) setError('Shared group context could not be loaded. RAIN will retry.'); }
    };
    load();
    const timer = setInterval(load, 3000);
    return () => { cancelled = true; clearInterval(timer); };
  }, [agentId]);
  if (error) return <p className="coworker-control-error" role="status">{error}</p>;
  if (!groups.length) return null;
  return <details className="group-awareness">
    <summary>Shared group context · {groups.length} {groups.length === 1 ? 'group' : 'groups'}</summary>
    <p>This agent knows its shared groups. Your direct messages stay in this conversation.</p>
    <div className="group-awareness-list">{groups.map(group => <details key={group.conversation.id}>
      <summary>{group.conversation.title} · {group.deleted_at ? 'Deleted · remembered' : !group.conversation.is_active ? 'Paused' : group.conversation.collaboration?.waiting_for_human ? 'Waiting for you' : 'Active'}</summary>
      {group.conversation.collaboration?.human_goal && <p>{group.conversation.collaboration.human_goal}</p>}
      {group.deleted_at && <p>Discussion is closed. Shared planning and results remain available to former members.</p>}
      <ol>{group.messages.slice(-8).map(message => <li key={message.id}><strong>{message.sender_name}: </strong>{message.text}</li>)}</ol>
      {group.messages.length > 8 && <p>Showing the latest 8 of {group.messages.length} retained messages.</p>}
    </details>)}</div>
  </details>;
}

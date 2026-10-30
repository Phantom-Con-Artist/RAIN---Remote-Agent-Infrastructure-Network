import React, { useEffect, useState } from 'react';
import { apiRequest } from '../api';
export function MachinePicker({ machine, onChange }: {
    machine: string;
    onChange: (value: string) => void;
}) {
    const [machines, setMachines] = useState<string[]>([]);
    const [error, setError] = useState('');
    useEffect(() => {
        let active = true;
        apiRequest<string[]>('/api/v1/agents/machines').then(r => {
            if (active)
                setMachines(r.data);
        }).catch(e => {
            if (active)
                setError(e.message);
        });
        return () => { active = false; };
    }, []);
    return <label className="machine-picker flex min-w-0 items-center gap-3 text-xs [&_select]:max-w-48 [&_select]:text-xs">Machine<select value={machine} onChange={e => onChange(e.target.value)}><option value="">This host</option>{machines.map(m => <option key={m}>{m}</option>)}</select>{error && <span role="alert">{error}</span>}</label>;
}

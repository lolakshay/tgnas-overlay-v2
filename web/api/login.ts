import { VercelRequest, VercelResponse } from '@vercel/node';
import { createSessionToken } from './_auth';

export default function handler(req: VercelRequest, res: VercelResponse) {
    if (req.method !== 'POST') {
        return res.status(405).json({ error: 'Method not allowed' });
    }

    const { password } = req.body;
    const expectedPassword = process.env.DASHBOARD_PASSWORD;

    if (!expectedPassword) {
        return res.status(500).json({ error: 'Server authentication is not configured' });
    }

    if (password !== expectedPassword) {
        return res.status(401).json({ error: 'Invalid password' });
    }

    const token = createSessionToken();
    res.setHeader(
        'Set-Cookie',
        `session=${token}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=604800`
    );
    return res.status(200).json({ success: true });
}

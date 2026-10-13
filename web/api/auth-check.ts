import { VercelRequest, VercelResponse } from '@vercel/node';
import { isAuthenticated } from './_auth';

export default function handler(req: VercelRequest, res: VercelResponse) {
    if (req.method !== 'GET') {
        return res.status(405).json({ error: 'Method not allowed' });
    }

    const authenticated = isAuthenticated(req);
    return res.status(200).json({ authenticated });
}

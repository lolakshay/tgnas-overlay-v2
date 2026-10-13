import { VercelRequest, VercelResponse } from '@vercel/node';
import { del } from '@vercel/blob';
import { isAuthenticated } from './_auth';

export default async function handler(req: VercelRequest, res: VercelResponse) {
    if (req.method !== 'DELETE') {
        return res.status(405).json({ error: 'Method not allowed' });
    }

    // Authenticate user
    if (!isAuthenticated(req)) {
        return res.status(401).json({ error: 'Unauthorized' });
    }

    const { id } = req.query;
    if (!id || typeof id !== 'string') {
        return res.status(400).json({ error: 'Screenshot ID is required' });
    }

    try {
        // Decode the ID to retrieve the original pathname
        const pathname = Buffer.from(id, 'base64url').toString('utf8');

        // Secure pathname validation
        if (!pathname.startsWith('screenshots/') || pathname.includes('..')) {
            return res.status(400).json({ error: 'Invalid screenshot ID path' });
        }

        // Delete from Vercel Blob
        await del(pathname);

        return res.status(200).json({ success: true });
    } catch (err: any) {
        return res.status(500).json({ error: 'Failed to delete screenshot: ' + err.message });
    }
}

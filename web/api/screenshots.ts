import { VercelRequest, VercelResponse } from '@vercel/node';
import { list } from '@vercel/blob';
import { isAuthenticated } from './_auth';

export default async function handler(req: VercelRequest, res: VercelResponse) {
    if (req.method !== 'GET') {
        return res.status(405).json({ error: 'Method not allowed' });
    }

    // Authenticate user
    if (!isAuthenticated(req)) {
        return res.status(401).json({ error: 'Unauthorized' });
    }

    try {
        const { blobs } = await list({ prefix: 'screenshots/' });

        const screenshots = blobs.map((blob) => {
            const id = Buffer.from(blob.pathname).toString('base64url');
            return {
                id,
                url: blob.url,
                createdAt: blob.uploadedAt.toISOString(),
                size: blob.size,
            };
        });

        // Sort newest-first
        screenshots.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());

        return res.status(200).json({ screenshots });
    } catch (err: any) {
        return res.status(500).json({ error: 'Failed to retrieve screenshots: ' + err.message });
    }
}

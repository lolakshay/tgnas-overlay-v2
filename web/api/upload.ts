import { VercelRequest, VercelResponse } from '@vercel/node';
import Busboy from 'busboy';
import crypto from 'crypto';
import { put } from '@vercel/blob';

export const config = {
    api: {
        bodyParser: false,
    },
};

export default function handler(req: VercelRequest, res: VercelResponse) {
    if (req.method !== 'POST') {
        return res.status(405).json({ error: 'Method not allowed' });
    }

    // 1. Validate authorization
    const authHeader = req.headers.authorization;
    const expectedToken = process.env.DEVICE_UPLOAD_TOKEN;
    if (!expectedToken) {
        return res.status(500).json({ error: 'Server upload authentication is not configured' });
    }
    if (!authHeader || authHeader !== `Bearer ${expectedToken}`) {
        return res.status(401).json({ error: 'Unauthorized' });
    }

    // 2. Validate content-length header early if available
    const contentLengthHeader = req.headers['content-length'];
    const maxLimit = parseInt(process.env.MAX_FILE_SIZE || '20971520', 10); // Default 20 MB
    if (contentLengthHeader) {
        const contentLength = parseInt(contentLengthHeader, 10);
        if (contentLength > maxLimit) {
            return res.status(413).json({ error: `File size exceeds the limit of ${maxLimit} bytes` });
        }
    }

    const busboy = Busboy({
        headers: req.headers,
        limits: {
            files: 1,
            fileSize: maxLimit,
        },
    });

    let fileReceived = false;
    let sizeLimitExceeded = false;
    let hasError = false;
    let errorMessage = '';
    let fileChunks: Buffer[] = [];
    let uploadPromise: Promise<any> | null = null;
    const now = new Date();

    busboy.on('file', (fieldname, file, info) => {
        // Determine info dynamically to support multiple busboy versions
        const filename = typeof info === 'object' ? info.filename : (arguments[2] as string);
        const mimeType = typeof info === 'object' ? info.mimeType : (arguments[4] as string);

        if (fieldname !== 'screenshot') {
            file.resume();
            return;
        }

        if (mimeType !== 'image/png') {
            file.resume();
            hasError = true;
            errorMessage = 'Only PNG image screenshots are supported';
            return;
        }

        file.on('data', (chunk) => {
            fileChunks.push(chunk);
        });

        file.on('limit', () => {
            sizeLimitExceeded = true;
        });

        file.on('end', () => {
            if (sizeLimitExceeded) {
                hasError = true;
                errorMessage = `File size exceeds the limit of ${maxLimit} bytes`;
                return;
            }
            fileReceived = true;

            // Create unique path: screenshots/YYYY/MM/DD/screenshot_HHMMSS_random.png
            const yyyy = now.getUTCFullYear();
            const mm = String(now.getUTCMonth() + 1).padStart(2, '0');
            const dd = String(now.getUTCDate()).padStart(2, '0');
            const hhmmss =
                String(now.getUTCHours()).padStart(2, '0') +
                String(now.getUTCMinutes()).padStart(2, '0') +
                String(now.getUTCSeconds()).padStart(2, '0');
            const randomSuffix = crypto.randomBytes(3).toString('hex');
            const pathname = `screenshots/${yyyy}/${mm}/${dd}/screenshot_${hhmmss}_${randomSuffix}.png`;

            uploadPromise = (async () => {
                const fileBuffer = Buffer.concat(fileChunks);
                const blob = await put(pathname, fileBuffer, {
                    access: 'public',
                    addRandomSuffix: false,
                });
                return blob;
            })();
        });
    });

    busboy.on('finish', async () => {
        if (hasError) {
            return res.status(400).json({ error: errorMessage });
        }
        if (sizeLimitExceeded) {
            return res.status(413).json({ error: 'File size limit exceeded' });
        }
        if (!fileReceived || !uploadPromise) {
            return res.status(400).json({ error: 'No screenshot file provided' });
        }

        try {
            const blob = await uploadPromise;
            const id = Buffer.from(blob.pathname).toString('base64url');
            return res.status(200).json({
                success: true,
                id,
                url: blob.url,
                createdAt: now.toISOString(),
            });
        } catch (err: any) {
            return res.status(500).json({ error: 'Upload failed: ' + err.message });
        }
    });

    req.pipe(busboy);
}

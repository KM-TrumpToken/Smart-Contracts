import { percentAmount, generateSigner, signerIdentity, createSignerFromKeypair } from '@metaplex-foundation/umi'
import { TokenStandard, createAndMint } from '@metaplex-foundation/mpl-token-metadata'
import { createUmi } from '@metaplex-foundation/umi-bundle-defaults';
import { mplCandyMachine } from "@metaplex-foundation/mpl-candy-machine";
import {ComputeBudgetProgram} from "@solana/web3.js";
import secret from './guideSecret.json' assert { type: 'json' };

const umi = createUmi('https://api.devnet.solana.com'); 
//Replace with your QuickNode RPC Endpoint

const userWallet = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(secret));
const userWalletSigner = createSignerFromKeypair(umi, userWallet);

const metadata = {
    name: "USDC",
    symbol: "USDC",
    uri: "https://ipfs.io/ipfs/QmdgnBH8F8s47gKdx8L6mtsbp3pXgzLAyXn6ZjsJs8vKc3",
};

const mint = generateSigner(umi);
umi.use(signerIdentity(userWalletSigner));
umi.use(mplCandyMachine())
const PRIORITY_FEE_INSTRUCTIONS = ComputeBudgetProgram.setComputeUnitPrice({microLamports: 3500000});
createAndMint(umi, {
    mint,
    authority: umi.identity,
    name: metadata.name,
    symbol: metadata.symbol,
    uri: metadata.uri,
    sellerFeeBasisPoints: percentAmount(0),
    decimals: 8,
    amount: 2100000000000000,
    tokenOwner: userWallet.publicKey,
    tokenStandard: TokenStandard.Fungible,
    }).sendAndConfirm(umi,{feePayerValue: PRIORITY_FEE_INSTRUCTIONS}).then(() => {
    console.log("Successfully minted 1 million tokens (", mint.publicKey, ")");
});
  

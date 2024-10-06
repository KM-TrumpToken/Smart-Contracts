const {
    Connection,
    clusterApiUrl,
    Keypair,
    PublicKey,
  } = require('@solana/web3.js');
  const { setAuthority, AuthorityType,TOKEN_PROGRAM_ID } = require('@solana/spl-token');
  
  (async () => {
    // Connection to the network
    const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');
  
    // Load your wallet keypair
    const payer = Keypair.fromSecretKey(Uint8Array.from());
  
  const mintAddress = new PublicKey("")
   const tx = await setAuthority(
        connection,
        payer,
        mintAddress,
        payer.publicKey,
        AuthorityType.MintTokens,
        null, // this sets the mint authority to null
    );
    console.log("tx",tx);
    
  })();
  
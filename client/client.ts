import BN from "bn.js";
import * as web3 from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import type { SalonBelleza } from "../target/types/salon_belleza";

// Configure the client to use the local cluster
anchor.setProvider(anchor.AnchorProvider.env());

const program = anchor.workspace.SalonBelleza as anchor.Program<SalonBelleza>;

(async () => {
  // 0) Wallet y balance (auto)
  if (!pg?.wallet?.publicKey) {
    throw new Error("No hay wallet activa en Playground.");
  }
  const ownerPubkey = program.provider.publicKey;
  console.log("Wallet activa:", ownerPubkey.toBase58());
  const balance = await program.provider.connection.getBalance(ownerPubkey);
  console.log(`Balance: ${balance / web3.LAMPORTS_PER_SOL} SOL`);

  // 1) Provider y Program desde Playground
  const provider = new anchor.AnchorProvider(
    program.provider.connection,
    pg.wallet,
    anchor.AnchorProvider.defaultOptions()
  );
  anchor.setProvider(provider);

  const program = program;
  if (!program) {
    throw new Error("program es undefined. Compila el proyecto primero (Build).");
  }
  const PROGRAM_ID = program.programId;

  // 2) PDA: seeds = ["salon_belleza", owner]
  const [salonPda] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("salon_belleza"), ownerPubkey.toBuffer()],
    PROGRAM_ID
  );
  console.log("Salón de Belleza PDA:", salonPda.toBase58());

  // 3) Listeners de eventos
  const subs = [];
  subs.push(
    await program.addEventListener("ServicioAgregado", (e, slot) => {
      console.log("Evento ServicioAgregado:", {
        nombre: e.nombre,
        precio: e.precio.toString(),
        slot,
      });
    })
  );
  subs.push(
    await program.addEventListener("ClienteRegistrado", (e, slot) => {
      console.log("Evento ClienteRegistrado:", {
        nombre: e.nombre,
        slot,
      });
    })
  );
  subs.push(
    await program.addEventListener("AtencionRegistrada", (e, slot) => {
      console.log("Evento AtencionRegistrada:", {
        cliente: e.clienteNombre,
        servicio: e.servicioNombre,
        cantidad: e.cantidad,
        total: e.total.toString(),
        slot,
      });
    })
  );

  // 4) Datos de prueba
  const NOMBRE_SALON = "Estética Glamour";
  const SERVICIO_NOMBRE = "Corte y Brushing";
  const SERVICIO_PRECIO_SOL = 0.002; // 0.002 SOL
  const SERVICIO_PRECIO_LAMPORTS = new anchor.BN(
    Math.floor(SERVICIO_PRECIO_SOL * web3.LAMPORTS_PER_SOL)
  );
  const CLIENTE_NOMBRE = "Camila";
  const ATENCION_CANTIDAD = 1; // u16

  // 5) crear_salon (crear solo si NO existe) — previene "already in use"
  {
    const accInfo = await program.provider.connection.getAccountInfo(salonPda);
    if (!accInfo) {
      console.log("-- crear_salon (no existe, creando)");
      const tx = await program.methods
        .crearSalon(NOMBRE_SALON)
        .accounts({
          owner: ownerPubkey,
          salon: salonPda,
          systemProgram: web3.SystemProgram.programId,
        })
        .rpc();
      console.log("crear_salon tx:", tx);
    } else {
      console.log("-- crear_salon: ya existe, continuando...");
    }
  }

  // 6) agregar_servicio (nota: ya no recibe stock, solo nombre y precio)
  console.log("-- agregar_servicio");
  await program.methods
    .agregarServicio(SERVICIO_NOMBRE, SERVICIO_PRECIO_LAMPORTS)
    .accounts({ owner: ownerPubkey, salon: salonPda })
    .rpc();

  // 7) registrar_cliente
  console.log("-- registrar_cliente");
  await program.methods
    .registrarCliente(CLIENTE_NOMBRE)
    .accounts({ owner: ownerPubkey, salon: salonPda })
    .rpc();

  // 8) registrar_atencion
  console.log("-- registrar_atencion");
  await program.methods
    .registrarAtencion(CLIENTE_NOMBRE, SERVICIO_NOMBRE, ATENCION_CANTIDAD)
    .accounts({ owner: ownerPubkey, salon: salonPda })
    .rpc();

  // 9) Leer estado
  const state = await program.account.salonBelleza.fetch(salonPda);
  console.log("Resumen estado:", {
    nombre: state.nombre,
    servicios: state.servicios.length,
    clientes: state.clientes.length,
    primerServicio: state.servicios[0]
      ? {
          nombre: state.servicios[0].nombre,
          precio: state.servicios[0].precio.toString(),
          realizados: state.servicios[0].realizados,
          disponible: state.servicios[0].disponible,
        }
      : null,
    primerCliente: state.clientes[0]
      ? {
          nombre: state.clientes[0].nombre,
          visitas: state.clientes[0].visitas,
          gastadoTotal: state.clientes[0].gastadoTotal.toString(),
        }
      : null,
  });

  // 10) Limpieza de listeners
  for (const id of subs) {
    await program.removeEventListener(id);
  }

  console.log("Listo.");
})().catch((err) => {
  console.error(err);
});
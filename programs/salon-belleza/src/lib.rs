/*
SALON DE BELLEZA

Proposito:
- Gestionar un salon de belleza por owner (PDA unica por propietario).
- Administrar servicios (cortes, peinados, manicure, etc.) y clientes, y registrar atenciones.
- Acceso: solo el owner puede mutar estado (check en handlers y constraints).

Modelo:
- SalonBelleza { owner: Pubkey, nombre: String(max 60), servicios: Vec<Servicio>(cap 40), clientes: Vec<Cliente>(cap 40) }
- Servicio { nombre: String(max 60), precio: u64(lamports), realizados: u32, disponible: bool }
- Cliente  { nombre: String(max 60), visitas: u32, gastado_total: u64(lamports) }

PDA:
- seeds = ["salon_belleza", owner] (direccion deterministica por propietario)
*/

use anchor_lang::prelude::*;

// Recuerda reemplazar este ID con el que te genere Solana Playground al hacer Build
declare_id!("11111111111111111111111111111111");

#[program]
pub mod salon_belleza {
    use super::*;

    /// Crea el salón de belleza (PDA).
    pub fn crear_salon(ctx: Context<NuevoSalon>, nombre: String) -> Result<()> {
        if nombre.len() > 60 {
            return Err(Errores::NombreMuyLargo.into());
        }

        let owner_id = ctx.accounts.owner.key();
        ctx.accounts.salon.set_inner(SalonBelleza {
            owner: owner_id,
            nombre,
            servicios: Vec::new(),
            clientes: Vec::new(),
        });
        Ok(())
    }

    // -------- Servicios --------

    /// Agrega un servicio (precio > 0, cap 40).
    pub fn agregar_servicio(
        ctx: Context<AccesoSalon>,
        nombre: String,
        precio: u64,
    ) -> Result<()> {
        if ctx.accounts.salon.owner != ctx.accounts.owner.key() {
            return Err(Errores::NoEresOwner.into());
        }
        if nombre.len() > 60 {
            return Err(Errores::NombreMuyLargo.into());
        }
        if ctx.accounts.salon.servicios.len() >= 40 {
            return Err(Errores::CapacidadDeServiciosLlena.into());
        }
        if precio == 0 {
            return Err(Errores::PrecioInvalido.into());
        }

        ctx.accounts.salon.servicios.push(Servicio {
            nombre: nombre.clone(),
            precio,
            realizados: 0,
            disponible: true,
        });

        let ts = Clock::get()?.unix_timestamp;
        emit!(ServicioAgregado {
            salon: ctx.accounts.salon.key(),
            owner: ctx.accounts.owner.key(),
            nombre,
            precio,
            timestamp: ts,
        });
        Ok(())
    }

    /// Elimina un servicio por nombre.
    pub fn eliminar_servicio(ctx: Context<AccesoSalon>, nombre: String) -> Result<()> {
        if ctx.accounts.salon.owner != ctx.accounts.owner.key() {
            return Err(Errores::NoEresOwner.into());
        }

        let servicios = &mut ctx.accounts.salon.servicios;
        if let Some(idx) = servicios.iter().position(|s| s.nombre == nombre) {
            servicios.remove(idx);

            let ts = Clock::get()?.unix_timestamp;
            emit!(ServicioEliminado {
                salon: ctx.accounts.salon.key(),
                owner: ctx.accounts.owner.key(),
                nombre,
                timestamp: ts,
            });
            Ok(())
        } else {
            Err(Errores::ServicioNoExiste.into())
        }
    }

    /// Alterna disponibilidad del servicio (ej. estilista ausente o sin insumos).
    pub fn alterar_servicio(ctx: Context<AccesoSalon>, nombre: String) -> Result<()> {
        if ctx.accounts.salon.owner != ctx.accounts.owner.key() {
            return Err(Errores::NoEresOwner.into());
        }

        let servicios = &mut ctx.accounts.salon.servicios;
        if let Some(idx) = servicios.iter().position(|s| s.nombre == nombre) {
            let nuevo = !servicios[idx].disponible;
            servicios[idx].disponible = nuevo;

            let ts = Clock::get()?.unix_timestamp;
            emit!(ServicioAlterado {
                salon: ctx.accounts.salon.key(),
                owner: ctx.accounts.owner.key(),
                nombre,
                disponible: nuevo,
                timestamp: ts,
            });
            Ok(())
        } else {
            Err(Errores::ServicioNoExiste.into())
        }
    }

    /// Actualiza el precio del servicio (lamports > 0).
    pub fn actualizar_precio(
        ctx: Context<AccesoSalon>,
        nombre: String,
        nuevo_precio: u64,
    ) -> Result<()> {
        if ctx.accounts.salon.owner != ctx.accounts.owner.key() {
            return Err(Errores::NoEresOwner.into());
        }
        if nuevo_precio == 0 {
            return Err(Errores::PrecioInvalido.into());
        }

        let servicios = &mut ctx.accounts.salon.servicios;
        if let Some(s) = servicios.iter_mut().find(|s| s.nombre == nombre) {
            s.precio = nuevo_precio;

            let ts = Clock::get()?.unix_timestamp;
            emit!(ServicioPrecioActualizado {
                salon: ctx.accounts.salon.key(),
                owner: ctx.accounts.owner.key(),
                nombre,
                precio: nuevo_precio,
                timestamp: ts,
            });
            Ok(())
        } else {
            Err(Errores::ServicioNoExiste.into())
        }
    }

    // -------- Clientes --------

    /// Registra un cliente (cap 40).
    pub fn registrar_cliente(ctx: Context<AccesoSalon>, nombre: String) -> Result<()> {
        if ctx.accounts.salon.owner != ctx.accounts.owner.key() {
            return Err(Errores::NoEresOwner.into());
        }
        if nombre.len() > 60 {
            return Err(Errores::NombreMuyLargo.into());
        }
        if ctx.accounts.salon.clientes.len() >= 40 {
            return Err(Errores::CapacidadDeClientesLlena.into());
        }

        ctx.accounts.salon.clientes.push(Cliente {
            nombre: nombre.clone(),
            visitas: 0,
            gastado_total: 0,
        });

        let ts = Clock::get()?.unix_timestamp;
        emit!(ClienteRegistrado {
            salon: ctx.accounts.salon.key(),
            owner: ctx.accounts.owner.key(),
            nombre,
            timestamp: ts,
        });
        Ok(())
    }

    // -------- Atenciones / Ventas --------

    /// Registra una atención (incrementa contador de servicios realizados y métricas del cliente).
    pub fn registrar_atencion(
        ctx: Context<AccesoSalon>,
        cliente_nombre: String,
        servicio_nombre: String,
        cantidad: u16,
    ) -> Result<()> {
        if ctx.accounts.salon.owner != ctx.accounts.owner.key() {
            return Err(Errores::NoEresOwner.into());
        }
        if cantidad == 0 {
            return Err(Errores::CantidadInvalida.into());
        }

        let salon_key = ctx.accounts.salon.key();
        let owner_key = ctx.accounts.owner.key();

        let cliente_idx = ctx
            .accounts
            .salon
            .clientes
            .iter()
            .position(|c| c.nombre == cliente_nombre)
            .ok_or(Errores::ClienteNoExiste)?;

        let servicio_idx = ctx
            .accounts
            .salon
            .servicios
            .iter()
            .position(|s| s.nombre == servicio_nombre)
            .ok_or(Errores::ServicioNoExiste)?;

        let precio_unitario: u64 = {
            let s_ref = &ctx.accounts.salon.servicios[servicio_idx];
            if !s_ref.disponible {
                return Err(Errores::ServicioNoDisponible.into());
            }
            s_ref.precio
        };

        let total_u128 = (precio_unitario as u128)
            .checked_mul(cantidad as u128)
            .ok_or(Errores::MontoOverflow)?;
        if total_u128 > u64::MAX as u128 {
            return Err(Errores::MontoOverflow.into());
        }
        let total = total_u128 as u64;

        // Actualizar estadísticas del servicio
        {
            let s_mut = &mut ctx.accounts.salon.servicios[servicio_idx];
            s_mut.realizados = s_mut.realizados.saturating_add(cantidad as u32);
        }

        // Actualizar métricas del cliente
        {
            let c_mut = &mut ctx.accounts.salon.clientes[cliente_idx];
            c_mut.visitas = c_mut.visitas.saturating_add(1);
            c_mut.gastado_total = c_mut.gastado_total.saturating_add(total);
        }

        let ts = Clock::get()?.unix_timestamp;
        emit!(AtencionRegistrada {
            salon: salon_key,
            owner: owner_key,
            cliente_nombre,
            servicio_nombre,
            cantidad,
            precio_unitario,
            total,
            timestamp: ts,
        });

        Ok(())
    }

    // -------- Lecturas --------

    pub fn leer_servicios(ctx: Context<LecturaSalon>) -> Result<()> {
        let count = ctx.accounts.salon.servicios.len();
        msg!("Total de servicios en catálogo: {}", count);
        Ok(())
    }

    pub fn leer_clientes(ctx: Context<LecturaSalon>) -> Result<()> {
        let count = ctx.accounts.salon.clientes.len();
        msg!("Total de clientes registrados: {}", count);
        Ok(())
    }
}

#[error_code]
pub enum Errores {
    #[msg("No eres el propietario de este salón")]
    NoEresOwner,
    #[msg("El servicio no existe")]
    ServicioNoExiste,
    #[msg("El servicio no está disponible actualmente")]
    ServicioNoDisponible,
    #[msg("El cliente no existe")]
    ClienteNoExiste,
    #[msg("Capacidad máxima de servicios alcanzada")]
    CapacidadDeServiciosLlena,
    #[msg("Capacidad máxima de clientes alcanzada")]
    CapacidadDeClientesLlena,
    #[msg("El nombre excede los 60 caracteres")]
    NombreMuyLargo,
    #[msg("El precio debe ser mayor a 0")]
    PrecioInvalido,
    #[msg("La cantidad debe ser mayor a 0")]
    CantidadInvalida,
    #[msg("Overflow en el monto calculado")]
    MontoOverflow,
}

#[account]
#[derive(InitSpace)]
pub struct SalonBelleza {
    pub owner: Pubkey,
    #[max_len(60)]
    pub nombre: String,
    #[max_len(40)]
    pub servicios: Vec<Servicio>,
    #[max_len(40)]
    pub clientes: Vec<Cliente>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace, PartialEq, Debug)]
pub struct Servicio {
    #[max_len(60)]
    pub nombre: String,
    pub precio: u64,
    pub realizados: u32,
    pub disponible: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace, PartialEq, Debug)]
pub struct Cliente {
    #[max_len(60)]
    pub nombre: String,
    pub visitas: u32,
    pub gastado_total: u64,
}

#[derive(Accounts)]
pub struct NuevoSalon<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = SalonBelleza::INIT_SPACE + 8,
        seeds = [b"salon_belleza", owner.key().as_ref()],
        bump
    )]
    pub salon: Account<'info, SalonBelleza>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AccesoSalon<'info> {
    pub owner: Signer<'info>,
    #[account(mut)]
    pub salon: Account<'info, SalonBelleza>,
}

#[derive(Accounts)]
pub struct LecturaSalon<'info> {
    pub salon: Account<'info, SalonBelleza>,
}

#[event]
pub struct ServicioAgregado {
    pub salon: Pubkey,
    pub owner: Pubkey,
    pub nombre: String,
    pub precio: u64,
    pub timestamp: i64,
}

#[event]
pub struct ServicioEliminado {
    pub salon: Pubkey,
    pub owner: Pubkey,
    pub nombre: String,
    pub timestamp: i64,
}

#[event]
pub struct ServicioAlterado {
    pub salon: Pubkey,
    pub owner: Pubkey,
    pub nombre: String,
    pub disponible: bool,
    pub timestamp: i64,
}

#[event]
pub struct ServicioPrecioActualizado {
    pub salon: Pubkey,
    pub owner: Pubkey,
    pub nombre: String,
    pub precio: u64,
    pub timestamp: i64,
}

#[event]
pub struct ClienteRegistrado {
    pub salon: Pubkey,
    pub owner: Pubkey,
    pub nombre: String,
    pub timestamp: i64,
}

#[event]
pub struct AtencionRegistrada {
    pub salon: Pubkey,
    pub owner: Pubkey,
    pub cliente_nombre: String,
    pub servicio_nombre: String,
    pub cantidad: u16,
    pub precio_unitario: u64,
    pub total: u64,
    pub timestamp: i64,
}
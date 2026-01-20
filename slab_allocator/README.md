# Slab Allocator

Implémentation minimale d'un allocateur slab en Rust (no_std).

**Auteurs :** Guillaume Houriez et Mohamed Hassan  
**Classe :** 4SI2

---

## Comment lancer le projet

### Prérequis
- Rust et Cargo installés ([rustup.rs](https://rustup.rs/))

### Cloner depuis le bundle
```bash
git clone slab_allocator.bundle slab_allocator
cd slab_allocator
```

### Compilation
```bash
# Compiler le projet
cargo build

# Compiler en mode release
cargo build --release
```

### Exécuter les tests
```bash
# Lancer tous les tests
cargo test

# Lancer les tests avec sortie détaillée
cargo test -- --nocapture

# Lancer un test spécifique
cargo test test_slab_creation
```

### Vérifications
```bash
# Vérifier le code sans compiler
cargo check

# Formater le code
cargo fmt

# Linter le code
cargo clippy
```

---

## Qu'est-ce qu'un Slab Allocator ?

Un allocateur slab est un mécanisme de gestion mémoire conçu pour l'allocation et la désallocation efficaces d'objets de taille uniforme. Il est largement utilisé dans les noyaux de systèmes d'exploitation, notamment dans le noyau Linux.

### Concepts clés

**Slab** : Un bloc contigu de mémoire divisé en morceaux de taille fixe (objets). Chaque slab contient plusieurs objets de même taille.

**Cache** : Une collection de slabs gérant des objets d'une taille spécifique. Lorsqu'un slab devient plein, l'allocateur en crée un nouveau.

**Free List** : Une liste chaînée qui trace les objets disponibles dans un slab pour une allocation rapide.

### Fonctionnement

1. **Initialisation** : Un slab est alloué et divisé en objets de taille égale. Tous les objets sont ajoutés à la free list.

2. **Allocation** : Lorsque de la mémoire est demandée, l'allocateur :
   - Vérifie la free list pour un objet disponible
   - Le retire de la free list
   - Retourne un pointeur vers l'objet

3. **Désallocation** : Lorsque la mémoire est libérée, l'allocateur :
   - Ajoute l'objet de retour à la free list
   - La mémoire reste allouée dans le slab pour réutilisation

4. **Gestion des Slabs** : Plusieurs slabs peuvent exister par cache. Lorsque tous les slabs sont pleins, un nouveau slab est alloué.

### Avantages

- **Allocation/désallocation rapide** : Opérations en O(1) grâce aux free lists
- **Fragmentation réduite** : Les objets sont de taille uniforme
- **Cache-friendly** : Les objets du même type sont stockés ensemble
- **Pas de surcharge de métadonnées** : La free list utilise l'espace des objets eux-mêmes

## Implémentation

Cette implémentation fournit :

- `Slab` : Gère un seul slab d'objets de taille fixe
- `SlabAllocator` : Gère plusieurs slabs pour une taille d'objet
- `SlabCache` : Gère plusieurs allocateurs pour différentes classes de taille (64, 256, 512 octets)

### Architecture

```
SlabCache
├── SmallAllocator (64 octets)
│   ├── Slab 1
│   ├── Slab 2
│   └── ...
├── MediumAllocator (256 octets)
│   └── Slab 1
└── LargeAllocator (512 octets)
    └── Slab 1
```

## Utilisation

```rust
use slab_allocator::{SlabCache, Layout};

let mut cache = SlabCache::new();
let layout = Layout::from_size_align(64, 8).unwrap();

let ptr = cache.allocate(layout).unwrap();
cache.deallocate(ptr, layout);
```

## Tests

Lancer les tests avec :
```bash
cargo test
```

Les tests couvrent :
- Création de slab
- Allocation et désallocation basique
- Allocations multiples
- Gestion des slabs pleins
- Vérification d'appartenance des pointeurs
- Allocateurs multi-slabs
- Cache avec différentes tailles
- Cas limites (taille nulle, objets trop grands)
- Alignement mémoire
- Réutilisation de la mémoire libérée

---

## L'Allocateur SLUB du Noyau Linux

Le noyau Linux utilise l'allocateur SLUB (Unqueued Slab Allocator), une évolution de l'allocateur SLAB original.

### SLUB vs SLAB

**Améliorations du SLUB** :
- Design simplifié sans files d'attente par CPU
- Meilleures performances pour les systèmes multi-cœurs
- Surcharge mémoire réduite
- Accès direct par CPU sans verrous dans le fast path

### Structure du SLUB

1. **kmem_cache** : Représente un cache pour des types d'objets spécifiques
2. **kmem_cache_cpu** : Données par CPU pour le fast path sans verrou
3. **kmem_cache_node** : Données par nœud NUMA pour le slow path
4. **Page** : Les slabs sont alloués comme des pages depuis le page allocator

### Fonctionnalités clés

**Fast Path** : Allocation sans verrou depuis la freelist par CPU utilisant des opérations cmpxchg.

**Slow Path** : Se replie sur les freelists au niveau des nœuds ou alloue de nouveaux slabs depuis le page allocator.

**Suivi des objets** : Stocke les métadonnées dans l'espace des objets inutilisés ou dans des zones séparées.

**Débogage** : Red-zoning, empoisonnement et traçage pour la détection de corruption mémoire.

### Processus d'allocation SLUB

```
1. Vérifier la freelist par CPU
   └─> Succès → Retourner l'objet (fast path)
   └─> Échec → Continuer à l'étape 2

2. Remplir la freelist par CPU depuis un slab partiel
   └─> Succès → Retourner l'objet
   └─> Échec → Continuer à l'étape 3

3. Allouer un nouveau slab depuis le page allocator
   └─> Initialiser les métadonnées du slab
   └─> Ajouter à la liste partielle
   └─> Retourner l'objet (slow path)
```

### Organisation mémoire

```
Page Slab :
+------------------+
| Objet 1          |
| Objet 2          |
| ...              |
| Objet N          |
| Espace libre     |
| Métadonnées slab | (en fin de page ou séparées)
+------------------+
```

### Considérations de sécurité

**Exploitation du tas** :
- Manipulation de la free list (UAF, double-free)
- Corruption des métadonnées
- Attaques par confusion de type

**Mitigations** :
- Obfuscation des pointeurs de freelist (encodage XOR)
- Ordre aléatoire de la freelist
- Vérifications renforcées des métadonnées
- SLAB_TYPESAFE_BY_RCU pour certains caches

### Caractéristiques de performance

- **Allocation** : ~10-50 cycles CPU (fast path)
- **Désallocation** : ~10-50 cycles CPU
- **Scalabilité** : Excellente sur les systèmes multi-cœurs
- **Fragmentation** : Faible pour les objets de même taille

### Exploitation du SLUB

Le SLUB allocator est une cible fréquente d'exploitation dans le contexte de la sécurité des noyaux :

**Vulnérabilités courantes** :
- **Use-After-Free (UAF)** : Réutilisation d'un objet après libération
- **Double-Free** : Libération multiple du même objet
- **Heap Overflow** : Dépassement de tampon vers les objets adjacents
- **Type Confusion** : Réutilisation d'un slab pour un type d'objet différent

**Techniques d'exploitation** :
- Contrôle de la freelist pour allouer des objets à des adresses prédictibles
- Spray du heap pour positionner des objets malveillants
- Corruption de pointeurs de fonction dans les structures du noyau
- Élévation de privilèges via des structures task_struct ou cred

**Protections modernes** :
- KASLR (Kernel Address Space Layout Randomization)
- Freelist randomization
- Hardened usercopy
- SLAB_FREELIST_HARDENED
- CONFIG_SLAB_MERGE_DEFAULT disabled

---

## Ressources

- [LWN : L'allocateur SLUB](https://lwn.net/Articles/229984/)
- [The Slab Allocator: An Object-Caching Kernel Memory Allocator](http://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.29.4759)
- [Documentation du noyau Linux](https://www.kernel.org/doc/html/latest/vm/slub.html)
- [Learning Rust With Entirely Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/)
- [Kernel Exploits : SLUB Allocator](https://argp.github.io/2012/01/03/linux-kernel-heap-exploitation/)

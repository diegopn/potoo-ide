use crate::languages::model::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectTemplate {
    Portugol20,
    GPortugol,
    Visualg,
    PotigolClassic,
    PotigolJerimum,
    Calango,
    Mapler,
}

impl ProjectTemplate {
    pub(crate) const ALL: [Self; 7] = [
        Self::GPortugol,
        Self::Visualg,
        Self::Portugol20,
        Self::Mapler,
        Self::PotigolClassic,
        Self::PotigolJerimum,
        Self::Calango,
    ];

    pub(crate) fn profile(self) -> &'static ProjectTemplateProfile {
        let profile = match self {
            Self::GPortugol => &TEMPLATES[0],
            Self::Visualg => &TEMPLATES[1],
            Self::Portugol20 => &TEMPLATES[2],
            Self::Mapler => &TEMPLATES[3],
            Self::PotigolClassic => &TEMPLATES[4],
            Self::PotigolJerimum => &TEMPLATES[5],
            Self::Calango => &TEMPLATES[6],
        };
        debug_assert_eq!(profile.template, self);
        profile
    }

    pub(crate) fn default_for_language(language: Language) -> Self {
        language.profile().default_template
    }

    pub(crate) fn language(self) -> Language {
        self.profile().language
    }

    pub(crate) fn group(self) -> ProjectTemplateGroup {
        self.profile().group
    }

    pub(crate) fn order(self) -> u32 {
        self.profile().order
    }

    pub(crate) fn file_name(self) -> &'static str {
        self.language().file_name()
    }

    pub(crate) fn sample(self) -> &'static str {
        self.profile().sample
    }

    pub(crate) fn new_file_sample(self) -> &'static str {
        self.profile().new_file_sample
    }

    pub(crate) fn in_group(group: ProjectTemplateGroup) -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|template| template.group() == group)
            .collect()
    }

    pub(crate) fn for_group_order(group: ProjectTemplateGroup, order: u32) -> Option<Self> {
        if group == ProjectTemplateGroup::Calango {
            return None;
        }
        Self::in_group(group)
            .into_iter()
            .find(|template| template.order() == order)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectTemplateGroup {
    Portugol,
    Potigol,
    Calango,
}

impl ProjectTemplateGroup {
    pub(crate) fn for_language(language: Language) -> Self {
        language.profile().default_template.group()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ProjectTemplateProfile {
    pub(crate) template: ProjectTemplate,
    pub(crate) language: Language,
    pub(crate) sample: &'static str,
    pub(crate) new_file_sample: &'static str,
    pub(crate) group: ProjectTemplateGroup,
    pub(crate) order: u32,
}

static TEMPLATES: [ProjectTemplateProfile; 7] = [
    ProjectTemplateProfile {
        template: ProjectTemplate::GPortugol,
        language: Language::GPortugol,
        sample: crate::languages::portugol::g_portugol::examples::SAMPLE,
        new_file_sample: "",
        group: ProjectTemplateGroup::Portugol,
        order: 0,
    },
    ProjectTemplateProfile {
        template: ProjectTemplate::Visualg,
        language: Language::Visualg,
        sample: crate::languages::portugol::visualg::examples::SAMPLE,
        new_file_sample: "",
        group: ProjectTemplateGroup::Portugol,
        order: 1,
    },
    ProjectTemplateProfile {
        template: ProjectTemplate::Portugol20,
        language: Language::Portugol,
        sample: crate::languages::portugol::studio_20::examples::SAMPLE,
        new_file_sample: "",
        group: ProjectTemplateGroup::Portugol,
        order: 2,
    },
    ProjectTemplateProfile {
        template: ProjectTemplate::Mapler,
        language: Language::Mapler,
        sample: crate::languages::portugol::mapler::examples::SAMPLE,
        new_file_sample: "",
        group: ProjectTemplateGroup::Portugol,
        order: 3,
    },
    ProjectTemplateProfile {
        template: ProjectTemplate::PotigolClassic,
        language: Language::Potigol,
        sample: crate::languages::potigol::classic::examples::SAMPLE,
        new_file_sample: "",
        group: ProjectTemplateGroup::Potigol,
        order: 0,
    },
    ProjectTemplateProfile {
        template: ProjectTemplate::PotigolJerimum,
        language: Language::Potigol,
        sample: crate::languages::potigol::jerimum::examples::SAMPLE,
        new_file_sample: crate::languages::potigol::jerimum::examples::SAMPLE,
        group: ProjectTemplateGroup::Potigol,
        order: 1,
    },
    ProjectTemplateProfile {
        template: ProjectTemplate::Calango,
        language: Language::Calango,
        sample: crate::languages::calango::examples::SAMPLE,
        new_file_sample: "",
        group: ProjectTemplateGroup::Calango,
        order: 0,
    },
];

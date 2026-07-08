export type AppearanceKey =
  | 'system'
  | 'light'
  | 'dark'
  | 'oled-black'
  | 'gentle-gecko'
  | 'black-knight'
  | 'diamond'
  | 'dreams'
  | 'paranoid'
  | 'red-velvet'
  | 'subspace'
  | 'tiefling'
  | 'vibes';

export type AppearancePalette = {
  background: string;
  alternateBackground: string;
  tertiaryBackground: string;
  text: string;
  disabledText: string;
  highlight: string;
  focusRing: string;
  highlightedText: string;
  positiveText: string;
  negativeText: string;
  neutralText: string;
};

export type AppearanceOption = {
  key: AppearanceKey;
  label: string;
  linsyncValue: number;
  palette: AppearancePalette;
};

export const APPEARANCE_OPTIONS: AppearanceOption[] = [
  {
    key: 'system',
    label: 'Follow system',
    linsyncValue: 0,
    palette: {
      background: '#F5F5F5',
      alternateBackground: '#ECEEF2',
      tertiaryBackground: '#E3E6EC',
      text: '#1A1A1A',
      disabledText: '#616161',
      highlight: '#2D7FF9',
      focusRing: '#2D7FF9',
      highlightedText: '#FFFFFF',
      positiveText: '#1FA862',
      negativeText: '#D93B3B',
      neutralText: '#E08319',
    },
  },
  {
    key: 'light',
    label: 'Light',
    linsyncValue: 1,
    palette: {
      background: '#F5F5F5',
      alternateBackground: '#ECEEF2',
      tertiaryBackground: '#E3E6EC',
      text: '#1A1A1A',
      disabledText: '#616161',
      highlight: '#2D7FF9',
      focusRing: '#2D7FF9',
      highlightedText: '#FFFFFF',
      positiveText: '#1FA862',
      negativeText: '#D93B3B',
      neutralText: '#E08319',
    },
  },
  {
    key: 'dark',
    label: 'Dark',
    linsyncValue: 2,
    palette: {
      background: '#181818',
      alternateBackground: '#292929',
      tertiaryBackground: '#343434',
      text: '#F5F5F5',
      disabledText: '#9e9e9e',
      highlight: '#2D7FF9',
      focusRing: '#2D7FF9',
      highlightedText: '#FFFFFF',
      positiveText: '#2DBE7A',
      negativeText: '#F05656',
      neutralText: '#FFA948',
    },
  },
  {
    key: 'oled-black',
    label: 'OLED Black',
    linsyncValue: 12,
    palette: {
      background: '#000000',
      alternateBackground: '#050505',
      tertiaryBackground: '#111111',
      text: '#F5F5F5',
      disabledText: '#808080',
      highlight: '#2D7FF9',
      focusRing: '#2D7FF9',
      highlightedText: '#FFFFFF',
      positiveText: '#2DBE7A',
      negativeText: '#F05656',
      neutralText: '#FFA948',
    },
  },
  {
    key: 'gentle-gecko',
    label: 'Gentle Gecko',
    linsyncValue: 3,
    palette: {
      background: '#000000',
      alternateBackground: '#003322',
      tertiaryBackground: '#00593D',
      text: '#FFFFFF',
      disabledText: '#B8D6CA',
      highlight: '#00B86B',
      focusRing: '#00B86B',
      highlightedText: '#FFFFFF',
      positiveText: '#00FF7F',
      negativeText: '#FF5050',
      neutralText: '#FFAA00',
    },
  },
  {
    key: 'black-knight',
    label: 'Black Knight',
    linsyncValue: 4,
    palette: {
      background: '#000000',
      alternateBackground: '#003366',
      tertiaryBackground: '#00478F',
      text: '#FFFFFF',
      disabledText: '#B8CCE0',
      highlight: '#0078D4',
      focusRing: '#0078D4',
      highlightedText: '#FFFFFF',
      positiveText: '#00FF7F',
      negativeText: '#FF5050',
      neutralText: '#FFAA00',
    },
  },
  {
    key: 'diamond',
    label: 'Diamond',
    linsyncValue: 5,
    palette: {
      background: '#2D5B67',
      alternateBackground: '#4F7F8C',
      tertiaryBackground: '#7CA2B1',
      text: '#B9DAE9',
      disabledText: '#91B0BC',
      highlight: '#A5C5D5',
      focusRing: '#A5C5D5',
      highlightedText: '#1A2D34',
      positiveText: '#C7F7D6',
      negativeText: '#FFD2D2',
      neutralText: '#FFE2A8',
    },
  },
  {
    key: 'dreams',
    label: 'Dreams',
    linsyncValue: 6,
    palette: {
      background: '#210B4B',
      alternateBackground: '#3F1C6D',
      tertiaryBackground: '#6A2A98',
      text: '#FF3D94',
      disabledText: '#B95D91',
      highlight: '#B5307E',
      focusRing: '#B5307E',
      highlightedText: '#FFFFFF',
      positiveText: '#8DFFB0',
      negativeText: '#FF8AB5',
      neutralText: '#FFD166',
    },
  },
  {
    key: 'paranoid',
    label: 'Paranoid',
    linsyncValue: 7,
    palette: {
      background: '#1D1D4E',
      alternateBackground: '#3F3F88',
      tertiaryBackground: '#5F5FBF',
      text: '#D2D2F4',
      disabledText: '#A2A2C8',
      highlight: '#9A9AE0',
      focusRing: '#9A9AE0',
      highlightedText: '#17173A',
      positiveText: '#BFF6D0',
      negativeText: '#FFD2D2',
      neutralText: '#FFE0A3',
    },
  },
  {
    key: 'red-velvet',
    label: 'Red Velvet',
    linsyncValue: 8,
    palette: {
      background: '#1A0F0F',
      alternateBackground: '#3C1414',
      tertiaryBackground: '#8B2323',
      text: '#FFDCDC',
      disabledText: '#C99B9B',
      highlight: '#DC3C3C',
      focusRing: '#DC3C3C',
      highlightedText: '#FFFFFF',
      positiveText: '#8DFFB0',
      negativeText: '#FF8A8A',
      neutralText: '#FFD166',
    },
  },
  {
    key: 'subspace',
    label: 'Subspace',
    linsyncValue: 9,
    palette: {
      background: '#2E1A47',
      alternateBackground: '#4A2A6A',
      tertiaryBackground: '#794B8B',
      text: '#E2C7E6',
      disabledText: '#B69CBC',
      highlight: '#B77BB4',
      focusRing: '#B77BB4',
      highlightedText: '#241338',
      positiveText: '#BAF4CB',
      negativeText: '#FFD2D2',
      neutralText: '#FFE0A3',
    },
  },
  {
    key: 'tiefling',
    label: 'Tiefling',
    linsyncValue: 10,
    palette: {
      background: '#3A0A4D',
      alternateBackground: '#711D9A',
      tertiaryBackground: '#A42DB4',
      text: '#F9C54E',
      disabledText: '#BD9440',
      highlight: '#FF5C8A',
      focusRing: '#FF5C8A',
      highlightedText: '#FFFFFF',
      positiveText: '#9DFFC0',
      negativeText: '#FF9BB5',
      neutralText: '#F9C54E',
    },
  },
  {
    key: 'vibes',
    label: 'Vibes',
    linsyncValue: 11,
    palette: {
      background: '#0F0F1E',
      alternateBackground: '#1E1E3C',
      tertiaryBackground: '#CC00FF',
      text: '#00FFCC',
      disabledText: '#66A89A',
      highlight: '#FFCC00',
      focusRing: '#FFCC00',
      highlightedText: '#111111',
      positiveText: '#00FF7F',
      negativeText: '#FF5050',
      neutralText: '#FFCC00',
    },
  },
];

export const APPEARANCE_KEY_MAP: Record<AppearanceKey, AppearanceOption> = Object.fromEntries(
  APPEARANCE_OPTIONS.map((option) => [option.key, option]),
) as Record<AppearanceKey, AppearanceOption>;

export const LINSYNC_VALUE_TO_KEY: Record<number, AppearanceKey> = Object.fromEntries(
  APPEARANCE_OPTIONS.map((option) => [option.linsyncValue, option.key]),
) as Record<number, AppearanceKey>;

export const LEGACY_KEY_MAP: Record<string, AppearanceKey> = {
  'oled_black': 'oled-black',
  'oled-black': 'oled-black',
  'gentle_gecko': 'gentle-gecko',
  'gentle-gecko': 'gentle-gecko',
  'black_knight': 'black-knight',
  'black-knight': 'black-knight',
  'red_velvet': 'red-velvet',
  'red-velvet': 'red-velvet',
  'high-contrast': 'black-knight',
  'high_contrast': 'black-knight',
  'highcontrast': 'black-knight',
};
